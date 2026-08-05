//! WBMusic 桌面应用入口库
//!
//! 基于 Tauri v2：前端 Vue 3 通过 WebView2 渲染，
//! 后端业务逻辑（音乐源接入）运行在本进程内。

mod commands;
mod services;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use commands::{AppState, ProviderRegistry};
use services::session::SessionStore;
use services::{NeteaseProvider, QqMusicProvider};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      // 初始化日志（写入应用数据目录 + 控制台）
      let log_dir = app.path().app_log_dir()?;
      std::fs::create_dir_all(&log_dir)?;
      let log_target = Target::new(TargetKind::Folder {
        path: log_dir,
        file_name: None,
      });
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .targets([log_target, Target::new(TargetKind::Stdout)])
          .build(),
      )?;

      // 加载持久化会话
      let data_dir = app.path().app_data_dir()?;
      std::fs::create_dir_all(&data_dir)?;
      let mut store = SessionStore::load(&data_dir);

      // 迁移旧版明文凭据（session.json 中可能残留 Cookie）到系统凭据库
      migrate_legacy_credentials(&store, &data_dir);

      // 从系统凭据库恢复凭据到内存会话
      if let Ok(Some(c)) = store.load_credential(services::MusicProviderKind::QqMusic) {
        store.qq_music.credential = Some(c);
      }
      if let Ok(Some(c)) = store.load_credential(services::MusicProviderKind::Netease) {
        store.netease.credential = Some(c);
      }

      // 构建音乐源提供者（从会话恢复登录态）
      let qq_music = QqMusicProvider::new(store.qq_music.clone());
      let netease = NeteaseProvider::new(store.netease.clone());

      // 注入全局状态
      app.manage(AppState {
        session_store: Arc::new(Mutex::new(store)),
        providers: ProviderRegistry {
          qq_music: Arc::new(qq_music),
          netease: Arc::new(netease),
        },
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::search_music,
      commands::login_music,
      commands::logout_music,
      commands::get_login_status,
      commands::get_track_url,
      commands::get_lyrics,
      commands::get_user_playlists,
      commands::get_playlist_detail,
      commands::get_cache_dir,
      commands::open_qr_login,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

/// 迁移旧版 session.json 中的明文凭据到系统凭据库
///
/// 旧版本把 Cookie 明文写在 session.json 的 `credential` 字段中。
/// 新版本该字段已标记为 `#[serde(skip)]`，反序列化时不会读取，
/// 因此这里直接从原始 JSON 中提取并迁移到系统凭据库。
///
/// 注意：只有所有凭据都迁移成功（或没有旧凭据）时才重写 session.json。
/// 若有凭据迁移失败则保留原文件，避免旧凭据丢失导致无法重试。
fn migrate_legacy_credentials(store: &SessionStore, data_dir: &std::path::Path) {
    use services::MusicProviderKind;

    let path = SessionStore::file_path(data_dir);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return;
    };

    let kinds = [
        (MusicProviderKind::QqMusic, "qq_music"),
        (MusicProviderKind::Netease, "netease"),
    ];

    let mut all_ok = true;
    let mut any_legacy = false;
    for (kind, key) in kinds {
        let legacy = json
            .get(key)
            .and_then(|v| v.get("credential"))
            .and_then(|v| v.as_str())
            .filter(|c| !c.is_empty());
        if let Some(cookie) = legacy {
            any_legacy = true;
            // 仅当凭据库中还没有该源的凭据时才迁移，避免覆盖新登录态
            let already_exists = store
                .load_credential(kind)
                .map(|c| c.is_some())
                .unwrap_or(false);
            if !already_exists {
                match store.save_credential(kind, cookie) {
                    Ok(()) => log::info!("已迁移 {} 的旧版凭据到系统凭据库", key),
                    Err(e) => {
                        all_ok = false;
                        log::warn!("迁移 {} 旧版凭据失败: {}", key, e);
                    }
                }
            }
        }
    }

    // 凭据全部迁移成功才重写 session.json（凭据字段不会再落盘）
    if any_legacy && all_ok {
        if let Err(e) = store.save(data_dir) {
            log::warn!("重写 session.json 失败: {}", e);
        }
    } else if any_legacy && !all_ok {
        log::warn!("存在迁移失败的凭据，保留 session.json 原文件以便下次重试");
    }
}
