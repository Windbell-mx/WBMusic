//! Tauri 命令层
//!
//! 将服务层能力通过 `#[tauri::command]` 暴露给前端 JS 调用。
//! 前端使用 `@tauri-apps/api` 的 `invoke()` 触发。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::services::session::{ProviderSession, SessionStore};
use crate::services::{
    LoginCredential, LoginStatus, MusicProvider, MusicProviderKind, Playlist, PlaylistDetail,
    SearchResult,
};

/// 应用全局状态
pub struct AppState {
    /// 会话存储（持久化到磁盘）
    pub session_store: Arc<Mutex<SessionStore>>,
    /// 各音乐源提供者
    pub providers: ProviderRegistry,
}

/// 音乐源提供者注册表
#[derive(Clone)]
pub struct ProviderRegistry {
    pub qq_music: Arc<dyn MusicProvider>,
    pub netease: Arc<dyn MusicProvider>,
}

impl ProviderRegistry {
    pub fn get(&self, kind: MusicProviderKind) -> Option<Arc<dyn MusicProvider>> {
        match kind {
            MusicProviderKind::QqMusic => Some(self.qq_music.clone()),
            MusicProviderKind::Netease => Some(self.netease.clone()),
        }
    }

    pub fn all(&self) -> Vec<Arc<dyn MusicProvider>> {
        vec![self.qq_music.clone(), self.netease.clone()]
    }
}

/// 搜索音乐
#[tauri::command]
pub async fn search_music(
    state: State<'_, AppState>,
    keyword: String,
    source: String,
    limit: Option<u32>,
) -> Result<SearchResult, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    let limit = limit.unwrap_or(30).min(100);
    provider.search(&keyword, limit).await
}

/// 登录音乐源
#[tauri::command]
pub async fn login_music(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source: String,
    credential: LoginCredential,
) -> Result<LoginStatus, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;

    let ok = provider.login(&credential).await?;
    if !ok {
        return Err("登录失败".into());
    }

    persist_login(&app, state.inner(), kind, credential_plain(&credential))
}

/// 登录成功后：更新会话存储 + 凭据写入系统凭据库 + 保存 session 到磁盘
fn persist_login(
    app: &tauri::AppHandle,
    state: &AppState,
    kind: MusicProviderKind,
    plain: Option<String>,
) -> Result<LoginStatus, String> {
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", kind.as_str()))?;
    let session = provider.login_status();

    // 更新会话存储
    let mut store = state
        .session_store
        .lock()
        .map_err(|_| "会话存储锁获取失败".to_string())?;
    let ps = ProviderSession {
        logged_in: session.logged_in,
        nickname: session.nickname.clone(),
        user_id: session.user_id.clone(),
        credential: plain.clone(),
    };
    match kind {
        MusicProviderKind::QqMusic => store.qq_music = ps,
        MusicProviderKind::Netease => store.netease = ps,
    }

    // 凭据写入系统凭据库（不落盘 session.json）；失败不阻断登录，仅告警
    if let Some(c) = plain {
        if let Err(e) = store.save_credential(kind, &c) {
            log::warn!("保存 {} 凭据到系统凭据库失败: {}", kind.as_str(), e);
        }
    }
    save_session(app, &store)?;
    Ok(session)
}

/// 退出登录音乐源
#[tauri::command]
pub async fn logout_music(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source: String,
) -> Result<(), String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    provider.logout().await?;

    let mut store = state
        .session_store
        .lock()
        .map_err(|_| "会话存储锁获取失败".to_string())?;
    match kind {
        MusicProviderKind::QqMusic => store.qq_music = ProviderSession::default(),
        MusicProviderKind::Netease => store.netease = ProviderSession::default(),
    }
    // 删除系统凭据库中的凭据；失败仅告警，不阻断登出
    if let Err(e) = store.delete_credential(kind) {
        log::warn!("删除 {} 系统凭据失败: {}", source, e);
    }
    save_session(&app, &store)?;
    Ok(())
}

/// 查询所有音乐源的登录状态
#[tauri::command]
pub fn get_login_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut map = serde_json::Map::new();
    for provider in state.providers.all() {
        let status = provider.login_status();
        map.insert(
            provider.kind().as_str().to_string(),
            serde_json::to_value(status).map_err(|e| e.to_string())?,
        );
    }
    Ok(serde_json::Value::Object(map))
}

/// 获取歌曲播放地址
#[tauri::command]
pub async fn get_track_url(
    state: State<'_, AppState>,
    source: String,
    track_id: String,
) -> Result<String, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    provider.get_track_url(&track_id).await
}

/// 获取歌词
#[tauri::command]
pub async fn get_lyrics(
    state: State<'_, AppState>,
    source: String,
    track_id: String,
) -> Result<Option<String>, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    provider.get_lyrics(&track_id).await
}

/// 获取当前登录用户的歌单列表
#[tauri::command]
pub async fn get_user_playlists(
    state: State<'_, AppState>,
    source: String,
) -> Result<Vec<Playlist>, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    provider.get_user_playlists().await
}

/// 获取歌单详情（含歌曲列表）
#[tauri::command]
pub async fn get_playlist_detail(
    state: State<'_, AppState>,
    source: String,
    playlist_id: String,
) -> Result<PlaylistDetail, String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let provider = state
        .providers
        .get(kind)
        .ok_or_else(|| format!("音乐源未注册: {}", source))?;
    provider.get_playlist_detail(&playlist_id).await
}

/// 从凭据中提取明文（用于持久化）
fn credential_plain(credential: &LoginCredential) -> Option<String> {
    match credential {
        LoginCredential::Cookie(c) => Some(c.clone()),
        LoginCredential::Token(t) => Some(t.clone()),
        LoginCredential::Password { username, password } => {
            // 账号密码登录的会话凭据由服务端返回，这里返回空
            log::warn!("密码登录暂不持久化凭据: {}", username);
            let _ = password;
            None
        }
    }
}

/// 获取应用缓存目录（完整绝对路径），不存在则创建
/// 默认使用安装位置（exe 所在目录）下的 cache 目录，避免占用 C 盘 AppData
#[tauri::command]
pub fn get_cache_dir(_app: tauri::AppHandle) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("获取可执行文件路径失败: {}", e))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "获取可执行文件所在目录失败".to_string())?;
    let dir = exe_dir.join("cache");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    Ok(dir.to_string_lossy().into_owned())
}

/// 保存会话到磁盘
fn save_session(app: &tauri::AppHandle, store: &SessionStore) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    store.save(&dir)
}

/// 生成注入到登录页的初始化脚本：
/// 对 QQ 音乐自动点击页面上带"登录"的链接以弹出扫码弹窗（选择器失败不影响手动操作）
fn qr_login_script(source: &str, auto_click_login: bool) -> String {
    let auto_click = if auto_click_login {
        r#"
  setTimeout(function () {
    try {
      var links = document.querySelectorAll('a, button, span, div');
      for (var i = 0; i < links.length; i++) {
        var t = (links[i].innerText || '').trim();
        if (t === '登录' || t === '请登录' || (t.length < 12 && t.indexOf('登录') >= 0)) {
          if (links[i].tagName === 'A' || links[i].tagName === 'BUTTON' || links[i].tagName === 'SPAN' || links[i].tagName === 'DIV') {
            links[i].click();
            break;
          }
        }
      }
    } catch (e) {}
  }, 2500);"#
    } else {
        ""
    };
    let _ = source;
    format!(
        r#"(function () {{{auto_click}
}})();"#,
        auto_click = auto_click,
    )
}

/// 打开扫码登录窗口（真实浏览器环境，绕过模拟请求的风控）
///
/// - QQ 音乐: 打开 y.qq.com 并自动弹出登录框（内含 QQ / 微信两种扫码方式）
/// - 网易云: 打开 music.163.com 登录页（默认扫码登录）
///
/// 登录窗口打开后，在独立线程中轮询该窗口的 cookie store
/// （可读取 HttpOnly Cookie），检测到关键登录 Cookie 即自动完成登录。
#[tauri::command]
pub async fn open_qr_login(app: tauri::AppHandle, source: String) -> Result<(), String> {
    let kind = MusicProviderKind::from_str(&source)
        .ok_or_else(|| format!("未知音乐源: {}", source))?;
    let (label, url, title, auto_click) = match kind {
        MusicProviderKind::QqMusic => (
            "login_qq",
            "https://y.qq.com/",
            "扫码登录 · QQ音乐（支持 QQ / 微信扫码）",
            true,
        ),
        MusicProviderKind::Netease => (
            "login_netease",
            "https://music.163.com/#/login",
            "扫码登录 · 网易云音乐",
            false,
        ),
    };

    // 已存在则聚焦，避免重复开窗
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.set_focus();
        return Ok(());
    }

    let url = tauri::Url::parse(url).map_err(|e| format!("登录页 URL 无效: {}", e))?;
    let script = qr_login_script(&source, auto_click);
    let nav_url = url.clone();
    let win = WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title(title)
        .inner_size(520.0, 720.0)
        .min_inner_size(420.0, 600.0)
        .resizable(true)
        .center()
        .initialization_script(&script)
        .build()
        .map_err(|e| format!("打开登录窗口失败: {}", e))?;

    // 独立线程处理登录窗口会话：
    // 1) 清除上次登录遗留的 WebView2 会话数据——扫码窗口是真实浏览器会话，会把登录 Cookie
    //    持久化到应用数据目录，应用内退出登录不会清除它，导致下次打开直接是已登录状态、
    //    无需扫码就自动登录
    // 2) 等待清除完成后重新加载登录页（页面需刷新才会变为未登录状态）
    // 3) 轮询捕获登录 Cookie
    // 线程选型：Windows 上 cookies_for_url 在同步命令/事件处理器（主线程）中调用会死锁（wry #583）；
    // 在 tokio 任务中阻塞 recv 会占用 worker 线程，且与窗口方法组合调用可能死锁（tauri #15504）。
    // 因此使用独立 std::thread：阻塞等待结果既不影响主线程，也不占用 tokio runtime。
    let app2 = app.clone();
    std::thread::spawn(move || {
        // 1. 清除浏览数据（clear_all_browsing_data 为非阻塞消息投递，由主线程执行；
        //    清空的是共享 WebView2 profile 的 Cookie/Storage，主窗口为本地应用不受影响）
        if let Some(w) = app2.get_webview_window(label) {
            if let Err(e) = w.clear_all_browsing_data() {
                log::debug!("清除登录窗口浏览数据失败: {}", e);
            }
        }
        // 2. 等待清除完成（ClearBrowsingData 是异步 COM 操作），再重新加载登录页
        std::thread::sleep(Duration::from_millis(2000));
        if let Some(w) = app2.get_webview_window(label) {
            if let Err(e) = w.navigate(nav_url.clone()) {
                log::debug!("重新加载登录页失败: {}", e);
            }
        }
        // 3. 轮询捕获登录 Cookie
        poll_qr_login(app2, kind);
    });
    let _ = win;
    Ok(())
}

/// 轮询扫码登录窗口的 cookie store，检测到关键登录 Cookie 后自动完成登录
///
/// 运行在独立线程（非主线程、非 tokio worker）：
/// cookies_for_url 内部会发送消息到主线程事件循环并阻塞等待结果，
/// 因此在独立线程中调用既不会冻结 UI，也不会占用 tokio worker。
fn poll_qr_login(app: tauri::AppHandle, kind: MusicProviderKind) {
    let (label, check_url, keys) = match kind {
        MusicProviderKind::QqMusic => (
            "login_qq",
            "https://y.qq.com/",
            &["qm_keyst", "uin"][..],
        ),
        MusicProviderKind::Netease => (
            "login_netease",
            "https://music.163.com/",
            &["MUSIC_U"][..],
        ),
    };
    let Ok(url) = tauri::Url::parse(check_url) else {
        return;
    };
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(600);
    loop {
        // 窗口被用户手动关闭则结束轮询
        // （与 cookie 读取分开调用，避免 #15504 的窗口方法 + cookies 组合死锁）
        if app.get_webview_window(label).is_none() {
            log::info!("扫码登录窗口已关闭，停止轮询: {}", kind.as_str());
            return;
        }
        // 读取该站点域下的 cookies（含 HttpOnly）
        let cookies = app
            .get_webview_window(label)
            .and_then(|w| w.cookies_for_url(url.clone()).ok());
        if let Some(cookies) = cookies {
            let cookie_str = cookies
                .iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; ");
            let all_present = keys.iter().all(|k| cookie_str.contains(&format!("{}=", k)));
            if all_present {
                log::info!("扫码登录捕获到关键 Cookie: {}", kind.as_str());
                // async 登录逻辑在独立线程的 block_on 中执行（不占用 tokio worker）
                tauri::async_runtime::block_on(async {
                    let state = app.state::<AppState>();
                    let Some(provider) = state.providers.get(kind) else {
                        return;
                    };
                    match provider
                        .login(&LoginCredential::Cookie(cookie_str.clone()))
                        .await
                    {
                        Ok(true) => {
                            if let Ok(session) =
                                persist_login(&app, state.inner(), kind, Some(cookie_str))
                            {
                                let _ = app.emit("qr-login-success", &session);
                            }
                            if let Some(w) = app.get_webview_window(label) {
                                let _ = w.close();
                            }
                        }
                        _ => {
                            log::warn!("扫码登录凭据校验失败: {}", kind.as_str());
                        }
                    }
                });
                return;
            }
        }
        if start.elapsed() > timeout {
            log::info!("扫码登录等待超时: {}", kind.as_str());
            return;
        }
        std::thread::sleep(Duration::from_millis(1500));
    }
}
