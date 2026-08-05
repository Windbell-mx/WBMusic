//! QQ 音乐接入服务
//!
//! 通过 Cookie（浏览器登录 y.qq.com 后复制）调用 QQ 音乐官方接口：
//!   - 搜索：POST https://u.y.qq.com/cgi-bin/musicu.fcg（无需登录）
//!   - 播放地址：POST musicu.fcg 模块 UrlGetVkey（登录后可播 VIP）
//!   - 歌词：GET https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

use super::music_provider::{
    LoginCredential, LoginStatus, MusicProvider, MusicProviderKind, Playlist, PlaylistDetail,
    SearchResult, Track,
};
use super::session::ProviderSession;

/// QQ 音乐 API 基础地址
const API_BASE: &str = "https://u.y.qq.com";
/// 匿名请求使用的默认 uid
const DEFAULT_UID: &str = "3931641530";

/// QQ 音乐提供者
#[derive(Debug, Clone)]
pub struct QqMusicProvider {
    kind: MusicProviderKind,
    /// HTTP 客户端（带 Cookie 存储）
    client: reqwest::Client,
    /// 登录会话
    session: Arc<Mutex<ProviderSession>>,
}

impl QqMusicProvider {
    pub fn new(session: ProviderSession) -> Self {
        let client = reqwest::Client::builder()
            // 注意：不能启用 cookie_store —— 它会用 cookie jar 中历史积累的
            // cookies 覆盖手动设置的 Cookie header，导致认证 Cookie 不完整
            .user_agent(concat!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ",
                "(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            ))
            .build()
            .expect("构建 HTTP 客户端失败");

        Self {
            kind: MusicProviderKind::QqMusic,
            client,
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// 从 Cookie 中解析 (uin, 登录key)。key 可取 qm_keyst / qqmusic_key / music_key
    fn parse_cookie(cookie: &str) -> Option<(String, String)> {
        let mut uin: Option<String> = None;
        let mut key: Option<String> = None;
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                match k {
                    "uin" => {
                        let raw = v.strip_prefix('o').unwrap_or(v);
                        uin = Some(raw.to_string());
                    }
                    "qm_keyst" | "qqmusic_key" | "music_key" => {
                        if !v.is_empty() {
                            key = Some(v.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        match (uin, key) {
            (Some(u), Some(k)) if !u.is_empty() && !k.is_empty() => Some((u, k)),
            _ => None,
        }
    }

    /// 当前会话的 (uin, key)，未登录返回默认 uid + 空 key
    fn session_cred(&self) -> Result<(String, String), String> {
        let guard = self
            .session
            .lock()
            .map_err(|_| "会话锁获取失败".to_string())?;
        let cookie = guard.credential.as_deref().unwrap_or("");
        let (uin, key) = Self::parse_cookie(cookie).unwrap_or_else(|| {
            (DEFAULT_UID.to_string(), String::new())
        });
        Ok((uin, key))
    }

    /// 构建基础 comm 参数
    fn comm(&self, ct: &str) -> Result<Value, String> {
        let (uin, key) = self.session_cred()?;
        let mut comm = json!({
            "cv": 13020508,
            "v": 13020508,
            "QIMEI36": "6c9d3cd110abca9b16311cee10001e717614",
            "ct": ct,
            "tmeAppID": "qqmusic",
            "format": "json",
            "inCharset": "utf-8",
            "outCharset": "utf-8",
            "uid": uin,
        });
        if !key.is_empty() {
            comm["qq"] = Value::String(uin);
            comm["authst"] = Value::String(key);
        }
        Ok(comm)
    }

    /// 构建带 Cookie 的请求头（从会话凭据读取）
    fn cookie_headers(&self) -> Result<HeaderMap, String> {
        let guard = self
            .session
            .lock()
            .map_err(|_| "会话锁获取失败".to_string())?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/"),
        );
        if let Some(cookie) = guard.credential.as_deref() {
            headers.insert(
                "Cookie",
                HeaderValue::from_str(cookie)
                    .map_err(|e| format!("Cookie 格式错误: {}", e))?,
            );
        }
        Ok(headers)
    }

    /// 获取当前登录用户的歌单列表
    ///
    /// 用户创建的歌单：fcg_user_created_diss（需登录态，否则只返回目录）
    /// 用户收藏的歌单：fcg_get_profile_order_asset (reqtype=3)
    async fn fetch_user_playlists(&self) -> Result<Vec<Playlist>, String> {
        // 必须已登录（解析到 uin + key）才可能拿到歌单
        let (uin, key) = self.session_cred()?;
        if key.is_empty() {
            return Err("请先登录 QQ 音乐后获取歌单".into());
        }
        let headers = self.cookie_headers()?;
        let mut playlists: Vec<Playlist> = Vec::new();

        // 1) 用户创建的歌单
        let created_url = format!(
            "https://c.y.qq.com/rsc/fcgi-bin/fcg_user_created_diss?hostUin=0&hostuin={}&sin=0&size=200&g_tk=5381&loginUin=0&format=json&inCharset=utf8&outCharset=utf-8&notice=0&platform=yqq.json&needNewCode=0",
            uin
        );
        let resp = self
            .client
            .get(created_url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| format!("获取用户歌单失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析用户歌单失败: {}", e))?;
        if j["code"].as_i64() == Some(0) {
            if let Some(list) = j["data"]["disslist"].as_array() {
                for d in list {
                    // 实测该接口返回的歌单没有 dissid 字段，ID 在 tid 中（dissid 仅作兼容）
                    let id = d["tid"]
                        .as_i64()
                        .or_else(|| d["dissid"].as_i64())
                        .unwrap_or(0)
                        .to_string();
                    // 跳过无 ID 的系统目录（如 QZone背景音乐/本地上传，tid=0）
                    if id == "0" {
                        continue;
                    }
                    playlists.push(Playlist {
                        id,
                        name: d["diss_name"].as_str().unwrap_or("未命名歌单").to_string(),
                        description: None,
                        cover_url: d["diss_cover"].as_str().map(|s| s.to_string()),
                        track_count: d["song_cnt"].as_u64().unwrap_or(0) as u32,
                        play_count: d["listen_num"].as_u64().unwrap_or(0),
                        source: MusicProviderKind::QqMusic,
                    });
                }
            }
        }

        // 2) 用户收藏的歌单（接口可能返回 4000，忽略错误）
        let fav_url = format!(
            "https://c.y.qq.com/fav/fcgi-bin/fcg_get_profile_order_asset.fcg?ct=20&cid=205360956&userid={}&reqtype=3&sin=0&ein=100&format=json&g_tk=5381&loginUin=0&hostUin=0&platform=yqq.json&needNewCode=0",
            uin
        );
        if let Ok(resp) = self.client.get(&fav_url).headers(headers).send().await {
            if let Ok(j) = resp.json::<Value>().await {
                if j["code"].as_i64() == Some(0) {
                    if let Some(list) = j["data"]["cdlist"].as_array() {
                        for d in list {
                            // 收藏接口同样兼容 tid / dissid
                            let id = d["tid"]
                                .as_i64()
                                .or_else(|| d["dissid"].as_i64())
                                .unwrap_or(0)
                                .to_string();
                            if id == "0" {
                                continue;
                            }
                            // 去重（收藏列表可能包含自己创建的）
                            if playlists.iter().any(|p| p.id == id) {
                                continue;
                            }
                            playlists.push(Playlist {
                                id,
                                name: d["dissname"].as_str().unwrap_or("未命名歌单").to_string(),
                                description: None,
                                cover_url: d["logo"]
                                    .as_str()
                                    .map(|s| s.to_string())
                                    .or_else(|| {
                                        d["cover"].as_str().map(|s| s.to_string())
                                    }),
                                track_count: d["songcnt"].as_u64().unwrap_or(0) as u32,
                                play_count: d["listennum"].as_u64().unwrap_or(0),
                                source: MusicProviderKind::QqMusic,
                            });
                        }
                    }
                }
            }
        }

        Ok(playlists)
    }

    /// 获取歌单详情（含歌曲列表）
    async fn fetch_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String> {
        let url = format!(
            "https://c.y.qq.com/qzone/fcg-bin/fcg_ucc_getcdinfo_byids_cp.fcg?type=1&utf8=1&disstid={}&format=json&inCharset=utf8&outCharset=utf-8&g_tk=5381&loginUin=0&hostUin=0&notice=0&platform=yqq&needNewCode=0",
            playlist_id
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/n/yqq/playlist"),
        );
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("获取歌单详情失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析歌单详情失败: {}", e))?;
        let cd = j["cdlist"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or("歌单不存在或已删除")?;
        let songs = cd["songlist"]
            .as_array()
            .ok_or("歌单详情响应格式异常")?;
        let tracks: Vec<Track> = songs
            .iter()
            .map(|s| {
                let mid = s["songmid"].as_str().unwrap_or("").to_string();
                let title = s["songname"].as_str().unwrap_or("未知歌曲").to_string();
                let artist = s["singer"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                let album = s["albumname"].as_str().map(|x| x.to_string());
                let duration = s["interval"].as_u64().map(|d| d as u32);
                let album_mid = s["albummid"].as_str().unwrap_or("");
                let cover_url = if album_mid.is_empty() {
                    None
                } else {
                    Some(format!(
                        "https://y.gtimg.cn/music/photo_new/T002R800x800M000{}.jpg",
                        album_mid
                    ))
                };
                Track {
                    id: mid,
                    title,
                    artist,
                    album,
                    duration,
                    cover_url,
                    source: MusicProviderKind::QqMusic,
                }
            })
            .collect();
        let track_count = tracks.len() as u32;
        Ok(PlaylistDetail {
            id: playlist_id.to_string(),
            name: cd["dissname"].as_str().unwrap_or("未命名歌单").to_string(),
            description: cd["desc"].as_str().map(|s| s.to_string()),
            cover_url: cd["logo"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| cd["cover"].as_str().map(|s| s.to_string())),
            track_count,
            tracks,
        })
    }
}

#[async_trait]
impl MusicProvider for QqMusicProvider {
    fn kind(&self) -> MusicProviderKind {
        self.kind
    }

    async fn login(&self, credential: &LoginCredential) -> Result<bool, String> {
        match credential {
            LoginCredential::Cookie(cookie) => {
                if cookie.is_empty() {
                    return Err("Cookie 不能为空".into());
                }
                // 必须包含 uin + qm_keyst/qqmusic_key 才是有效登录态
                let Some((uin, _key)) = Self::parse_cookie(cookie) else {
                    return Err("Cookie 缺少 uin / qm_keyst，请确认已登录 y.qq.com 后复制完整 Cookie".into());
                };
                let mut guard = self
                    .session
                    .lock()
                    .map_err(|_| "会话锁获取失败".to_string())?;
                guard.logged_in = true;
                guard.credential = Some(cookie.clone());
                guard.nickname = Some(format!("QQ用户 {}", uin));
                guard.user_id = Some(uin);
                Ok(true)
            }
            LoginCredential::Password { .. } | LoginCredential::Token(_) => {
                Err("QQ 音乐当前仅支持 Cookie 方式登录（浏览器登录后复制 Cookie）".into())
            }
        }
    }

    async fn logout(&self) -> Result<(), String> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| "会话锁获取失败".to_string())?;
        guard.logged_in = false;
        guard.credential = None;
        guard.nickname = None;
        guard.user_id = None;
        Ok(())
    }

    fn login_status(&self) -> LoginStatus {
        let guard = self.session.lock().unwrap_or_else(|poisoned| {
            poisoned.into_inner()
        });
        LoginStatus {
            logged_in: guard.logged_in,
            nickname: guard.nickname.clone(),
            user_id: guard.user_id.clone(),
        }
    }

    async fn search(&self, keyword: &str, limit: u32) -> Result<SearchResult, String> {
        // 搜索无需登录，匿名可用
        let limit = limit.clamp(1, 30);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("获取时间戳失败: {}", e))?
            .as_millis() as u64;
        let body = json!({
            "comm": self.comm("11")?,
            "music.search.SearchCgiService.DoSearchForQQMusicMobile": {
                "module": "music.search.SearchCgiService",
                "method": "DoSearchForQQMusicMobile",
                "param": {
                    "searchid": ts * 1_000_000,
                    "query": keyword,
                    "search_type": 0,
                    "num_per_page": limit,
                    "page_num": 1,
                    "highlight": 1,
                    "grp": 1,
                }
            }
        });
        let resp = self
            .client
            .post(format!("{}/cgi-bin/musicu.fcg", API_BASE))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("搜索请求失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析搜索结果失败: {}", e))?;
        let data = &j["music.search.SearchCgiService.DoSearchForQQMusicMobile"]["data"];
        let songs = data["body"]["item_song"]
            .as_array()
            .ok_or("搜索响应格式异常")?;
        let total = data["body"]["total_num"].as_u64().unwrap_or(songs.len() as u64) as usize;

        let tracks: Vec<Track> = songs
            .iter()
            .map(|s| {
                let mid = s["mid"].as_str().unwrap_or("").to_string();
                let title = s["name"].as_str().unwrap_or("未知歌曲").to_string();
                let artist = s["singer"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                let album_mid = s["album"]["mid"].as_str().unwrap_or("");
                let album = s["album"]["name"].as_str().map(|x| x.to_string());
                let duration = s["interval"].as_u64().map(|d| d as u32);
                // 封面：https://y.gtimg.cn/music/photo_new/T002R800x800M000{albummid}.jpg
                let cover_url = if album_mid.is_empty() {
                    None
                } else {
                    Some(format!(
                        "https://y.gtimg.cn/music/photo_new/T002R800x800M000{}.jpg",
                        album_mid
                    ))
                };
                Track {
                    id: mid,
                    title,
                    artist,
                    album,
                    duration,
                    cover_url,
                    source: MusicProviderKind::QqMusic,
                }
            })
            .collect();

        Ok(SearchResult { tracks, total })
    }

    async fn get_track_url(&self, track_id: &str) -> Result<String, String> {
        // 通过 UrlGetVkey 获取播放地址；登录（VIP）后返回完整音质
        let body = json!({
            "comm": self.comm("19")?,
            "music.vkey.GetVkey.UrlGetVkey": {
                "module": "music.vkey.GetVkey",
                "method": "UrlGetVkey",
                "param": {
                    "filename": [format!("M500{}{}.mp3", track_id, track_id)],
                    "guid": "1234567890abcdef1234567890abcdef",
                    "songmid": [track_id],
                    "songtype": [0],
                }
            }
        });
        let resp = self
            .client
            .post(format!("{}/cgi-bin/musicu.fcg", API_BASE))
            .headers(self.cookie_headers()?)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("获取播放地址失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析播放地址失败: {}", e))?;
        let info = &j["music.vkey.GetVkey.UrlGetVkey"]["data"]["midurlinfo"][0];
        let purl = info["purl"].as_str().unwrap_or("");
        if purl.is_empty() {
            let result = info["result"].as_i64().unwrap_or(-1);
            return Err(format!(
                "该歌曲为 VIP 专享（result={}），登录 VIP 账号后可播放",
                result
            ));
        }
        Ok(format!("https://isure.stream.qqmusic.qq.com/{}", purl))
    }

    async fn get_lyrics(&self, track_id: &str) -> Result<Option<String>, String> {
        // 歌词接口：lyric 字段为 base64 编码的 LRC
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("获取时间戳失败: {}", e))?
            .as_millis();
        let url = format!(
            "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?songmid={}&format=json&pcachetime={}&g_tk=5381&loginUin=0&hostUin=0&inCharset=utf8&outCharset=utf-8&notice=0&platform=yqq&needNewCode=0",
            track_id, ts
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com"),
        );
        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("获取歌词失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析歌词失败: {}", e))?;
        let lyric_b64 = j["lyric"].as_str().unwrap_or("");
        if lyric_b64.is_empty() {
            return Ok(None);
        }
        let decoded = BASE64
            .decode(lyric_b64)
            .map_err(|e| format!("歌词解码失败: {}", e))?;
        let lrc = String::from_utf8_lossy(&decoded).to_string();
        if lrc.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(lrc))
        }
    }

    async fn get_user_playlists(&self) -> Result<Vec<Playlist>, String> {
        self.fetch_user_playlists().await
    }

    async fn get_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String> {
        self.fetch_playlist_detail(playlist_id).await
    }
}
