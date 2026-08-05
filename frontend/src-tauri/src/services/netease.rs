//! 网易云音乐接入服务
//!
//! 用户登录网页版 music.163.com 后复制 Cookie，应用使用该 Cookie
//! 调用网易云官方网页接口（搜索 / 播放地址 / 歌词 / 用户信息），
//! 无需逆向 weapi 加密。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

use super::music_provider::{
    LoginCredential, LoginStatus, MusicProvider, MusicProviderKind, Playlist, PlaylistDetail,
    SearchResult, Track,
};
use super::session::ProviderSession;

/// 网易云音乐 API 基础地址
const API_BASE: &str = "https://music.163.com";

/// 网易云音乐提供者
#[derive(Debug, Clone)]
pub struct NeteaseProvider {
    kind: MusicProviderKind,
    /// HTTP 客户端（带 Cookie 存储）
    client: reqwest::Client,
    /// 登录会话
    session: Arc<Mutex<ProviderSession>>,
}

impl NeteaseProvider {
    pub fn new(session: ProviderSession) -> Self {
        let client = reqwest::Client::builder()
            // 注意：不能启用 cookie_store —— 它会用 cookie jar 中历史积累的
            // cookies 覆盖手动设置的 Cookie header，导致认证 Cookie 不完整
            // 强制 HTTP/1.1：网易云接口在 HTTP/2 下返回业务 code=400
            .http1_only()
            .user_agent(concat!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ",
                "(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
            ))
            .build()
            .expect("构建 HTTP 客户端失败");

        Self {
            kind: MusicProviderKind::Netease,
            client,
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// 基础请求头（Referer 必需，否则接口可能拒绝）
    fn base_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://music.163.com/"),
        );
        headers
    }

    /// 构建带 Cookie 的请求头（从会话凭据读取）
    fn cookie_headers(&self) -> Result<HeaderMap, String> {
        let guard = self
            .session
            .lock()
            .map_err(|_| "会话锁获取失败".to_string())?;
        let mut headers = self.base_headers();
        if let Some(cookie) = guard.credential.as_deref() {
            headers.insert(
                "Cookie",
                HeaderValue::from_str(cookie)
                    .map_err(|e| format!("Cookie 格式错误: {}", e))?,
            );
        }
        Ok(headers)
    }

    /// 使用指定 Cookie 构建请求头（登录验证时用，此时新 Cookie 尚未写入 session）
    fn cookie_headers_with(&self, cookie: &str) -> Result<HeaderMap, String> {
        let mut headers = self.base_headers();
        headers.insert(
            "Cookie",
            HeaderValue::from_str(cookie)
                .map_err(|e| format!("Cookie 格式错误: {}", e))?,
        );
        Ok(headers)
    }

    /// 校验 Cookie 有效性，成功返回 (昵称, 用户ID)
    /// `explicit_cookie` 为 Some 时用该值验证（登录流程），为 None 时用 session 中已保存的凭据
    async fn validate_cookie(&self, explicit_cookie: Option<&str>) -> Result<(String, String), String> {
        let headers = match explicit_cookie {
            Some(cookie) => self.cookie_headers_with(cookie)?,
            None => self.cookie_headers()?,
        };
        let resp = self
            .client
            .get(format!("{}/api/nuser/account/get", API_BASE))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求用户信息失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析用户信息失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            return Err(format!("Cookie 校验失败 (code={})", code));
        }
        let profile = &j["profile"];
        if profile.is_null() {
            return Err("Cookie 无效或已过期，请重新登录网页版 music.163.com 后复制新 Cookie".into());
        }
        let nickname = profile["nickname"]
            .as_str()
            .unwrap_or("网易云用户")
            .to_string();
        let uid = profile["userId"].as_i64().unwrap_or(0).to_string();
        Ok((nickname, uid))
    }

    /// 批量获取歌曲封面（song/detail 接口）
    async fn fetch_covers(&self, ids: &[String]) -> Result<HashMap<String, String>, String> {
        let mut covers = HashMap::new();
        if ids.is_empty() {
            return Ok(covers);
        }
        // ids 需编码为 URL 数组形式：[id1,id2]
        let encoded = format!("%5B{}%5D", ids.join("%2C"));
        let resp = self
            .client
            .get(format!("{}/api/song/detail/?ids={}&csrf_token=", API_BASE, encoded))
            .headers(self.base_headers())
            .send()
            .await
            .map_err(|e| format!("请求歌曲详情失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析歌曲详情失败: {}", e))?;
        if let Some(songs) = j["songs"].as_array() {
            for s in songs {
                if let Some(id) = s["id"].as_i64() {
                    if let Some(pic) = s["album"]["picUrl"].as_str() {
                        covers.insert(id.to_string(), pic.to_string());
                    }
                }
            }
        }
        Ok(covers)
    }

    /// 获取当前登录用户的歌单列表
    async fn fetch_user_playlists(&self) -> Result<Vec<Playlist>, String> {
        let (_nickname, uid) = self
            .validate_cookie(None)
            .await
            .map_err(|e| format!("获取用户信息失败（请确认已登录网易云）: {}", e))?;
        let resp = self
            .client
            .get(format!(
                "{}/api/user/playlist?uid={}&limit=30&offset=0&includeVideo=true",
                API_BASE, uid
            ))
            .headers(self.cookie_headers()?)
            .send()
            .await
            .map_err(|e| format!("获取用户歌单失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析用户歌单失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            return Err(format!("获取用户歌单失败 (code={})", code));
        }
        let playlists = j["playlist"]
            .as_array()
            .ok_or("用户歌单响应格式异常")?;
        let mut result = Vec::new();
        for p in playlists {
            // 跳过 0 首歌的空目录
            let track_count = p["trackCount"].as_u64().unwrap_or(0) as u32;
            let id = p["id"].as_i64().unwrap_or(0).to_string();
            if id == "0" {
                continue;
            }
            result.push(Playlist {
                id,
                name: p["name"].as_str().unwrap_or("未命名歌单").to_string(),
                description: p["description"].as_str().map(|s| s.to_string()),
                cover_url: p["coverImgUrl"].as_str().map(|s| s.to_string()),
                track_count,
                play_count: p["playCount"].as_u64().unwrap_or(0),
                source: MusicProviderKind::Netease,
            });
        }
        Ok(result)
    }

    /// 获取歌单详情（含歌曲列表）
    ///
    /// 注意：`/api/v6/playlist/detail` 的 `tracks` 字段最多只返回前 10 首，
    /// 完整歌曲 ID 在 `trackIds` 中。因此这里先取 trackIds，再用
    /// `/api/v3/song/detail` 批量拉取全部歌曲详情，保证歌单歌曲完整。
    async fn fetch_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String> {
        let resp = self
            .client
            .get(format!(
                "{}/api/v6/playlist/detail?id={}",
                API_BASE, playlist_id
            ))
            .headers(self.cookie_headers()?)
            .send()
            .await
            .map_err(|e| format!("获取歌单详情失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析歌单详情失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            return Err(format!("获取歌单详情失败 (code={})", code));
        }
        let playlist = &j["playlist"];
        if playlist.is_null() {
            return Err("歌单不存在或已删除".into());
        }

        // 1. 提取完整歌曲 ID 列表（trackIds）
        let track_ids: Vec<String> = playlist["trackIds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["id"].as_i64())
                    .map(|id| id.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // 2. 先用详情接口自带的前若干首（tracks 字段），再补全剩余
        let mut tracks: Vec<Track> = playlist["tracks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| {
                        let id = s["id"].as_i64().unwrap_or(0).to_string();
                        let title = s["name"].as_str().unwrap_or("未知歌曲").to_string();
                        let artist = s["ar"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x["name"].as_str())
                                    .collect::<Vec<_>>()
                                    .join("/")
                            })
                            .unwrap_or_default();
                        let album = s["al"]["name"].as_str().map(|x| x.to_string());
                        let duration = s["dt"].as_u64().map(|d| (d / 1000) as u32);
                        let cover_url = s["al"]["picUrl"].as_str().map(|x| x.to_string());
                        Track {
                            id,
                            title,
                            artist,
                            album,
                            duration,
                            cover_url,
                            source: MusicProviderKind::Netease,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 3. 需要补全的 ID：trackIds 中尚未在 tracks 里的
        let mut existing: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        let missing: Vec<&String> = track_ids
            .iter()
            .filter(|id| !existing.contains(*id))
            .collect();
        if !missing.is_empty() {
            // 批量拉取（实测网易云 song/detail 单批上限约 200，超过返回 code=400）
            for chunk in missing.chunks(200) {
                // c 必须是 JSON 数组字符串，如 [{"id":1},{"id":2}]（带外层方括号）
                let c = chunk
                    .iter()
                    .map(|id| format!("{{\"id\":{}}}", id))
                    .collect::<Vec<_>>()
                    .join(",");
                let c = format!("[{}]", c);
                // c 含 { } " : , 等保留字符，必须 URL 编码
                let url = reqwest::Url::parse_with_params(
                    &format!("{}/api/v3/song/detail", API_BASE),
                    &[("c", &c)],
                )
                .map_err(|e| format!("构建歌曲详情请求失败: {}", e))?;
                let resp = self
                    .client
                    .get(url)
                    .headers(self.cookie_headers()?)
                    .send()
                    .await
                    .map_err(|e| format!("获取歌单歌曲详情失败: {}", e))?;
                let sj: Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("解析歌单歌曲详情失败: {}", e))?;
                if let Some(songs) = sj["songs"].as_array() {
                    for s in songs {
                        let id = s["id"].as_i64().unwrap_or(0).to_string();
                        if id == "0" || existing.contains(&id) {
                            continue;
                        }
                        let title = s["name"].as_str().unwrap_or("未知歌曲").to_string();
                        let artist = s["ar"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x["name"].as_str())
                                    .collect::<Vec<_>>()
                                    .join("/")
                            })
                            .unwrap_or_default();
                        let album = s["al"]["name"].as_str().map(|x| x.to_string());
                        let duration = s["dt"].as_u64().map(|d| (d / 1000) as u32);
                        let cover_url = s["al"]["picUrl"].as_str().map(|x| x.to_string());
                        existing.insert(id.clone());
                        tracks.push(Track {
                            id,
                            title,
                            artist,
                            album,
                            duration,
                            cover_url,
                            source: MusicProviderKind::Netease,
                        });
                    }
                }
            }
        }

        // 4. 歌单总歌曲数：优先取 trackCount 字段（比 trackIds 更准）
        let track_count = playlist["trackCount"]
            .as_u64()
            .unwrap_or(track_ids.len() as u64) as u32;

        Ok(PlaylistDetail {
            id: playlist_id.to_string(),
            name: playlist["name"].as_str().unwrap_or("未命名歌单").to_string(),
            description: playlist["description"].as_str().map(|s| s.to_string()),
            cover_url: playlist["coverImgUrl"].as_str().map(|s| s.to_string()),
            track_count,
            tracks,
        })
    }
}

#[async_trait]
impl MusicProvider for NeteaseProvider {
    fn kind(&self) -> MusicProviderKind {
        self.kind
    }

    async fn login(&self, credential: &LoginCredential) -> Result<bool, String> {
        match credential {
            LoginCredential::Cookie(cookie) => {
                if cookie.is_empty() {
                    return Err("Cookie 不能为空".into());
                }
                // 先校验 Cookie 有效性并获取用户信息（用新 Cookie 验证，而非 session 旧凭据）
                let (nickname, uid) = self.validate_cookie(Some(cookie)).await?;
                let mut guard = self
                    .session
                    .lock()
                    .map_err(|_| "会话锁获取失败".to_string())?;
                guard.logged_in = true;
                guard.credential = Some(cookie.clone());
                guard.nickname = Some(nickname);
                guard.user_id = Some(uid);
                Ok(true)
            }
            LoginCredential::Password { .. } | LoginCredential::Token(_) => {
                Err("网易云音乐当前仅支持 Cookie 方式登录（浏览器登录后复制 Cookie）".into())
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
        // 搜索接口无需登录，匿名可用
        let limit = limit.clamp(1, 50);
        let resp = self
            .client
            .post(format!("{}/api/search/get/web", API_BASE))
            .headers(self.base_headers())
            .form(&[
                ("s", keyword),
                ("type", "1"),
                ("offset", "0"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("搜索请求失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析搜索结果失败: {}", e))?;
        let songs = j["result"]["songs"]
            .as_array()
            .ok_or("搜索结果格式异常")?;
        let total = j["result"]["songCount"]
            .as_u64()
            .unwrap_or(songs.len() as u64) as usize;

        // 批量获取封面
        let ids: Vec<String> = songs
            .iter()
            .filter_map(|s| s["id"].as_i64().map(|i| i.to_string()))
            .collect();
        let covers = self.fetch_covers(&ids).await.unwrap_or_default();

        let tracks: Vec<Track> = songs
            .iter()
            .map(|s| {
                let id = s["id"].as_i64().unwrap_or(0).to_string();
                let title = s["name"].as_str().unwrap_or("未知歌曲").to_string();
                let artist = s["artists"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                let album = s["album"]["name"].as_str().map(|x| x.to_string());
                let duration = s["duration"].as_u64().map(|d| (d / 1000) as u32);
                let cover_url = covers.get(&id).cloned();
                Track {
                    id,
                    title,
                    artist,
                    album,
                    duration,
                    cover_url,
                    source: MusicProviderKind::Netease,
                }
            })
            .collect();

        Ok(SearchResult { tracks, total })
    }

    async fn get_track_url(&self, track_id: &str) -> Result<String, String> {
        // 免费歌曲匿名可获取；VIP 歌曲需带登录 Cookie（用户账号为 VIP 才可播放）
        let ids = format!("%5B{}%5D", track_id);
        let resp = self
            .client
            .get(format!(
                "{}/api/song/enhance/player/url?ids={}&br=320000&csrf_token=",
                API_BASE, ids
            ))
            .headers(self.cookie_headers()?)
            .send()
            .await
            .map_err(|e| format!("获取播放地址失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析播放地址失败: {}", e))?;
        let data = j["data"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or("播放地址响应格式异常")?;
        let code = data["code"].as_i64().unwrap_or(-1);
        let url = data["url"].as_str().unwrap_or("");
        if code != 200 || url.is_empty() {
            return Err("该歌曲需 VIP 或登录后播放（Cookie 无效/过期，或无版权）".into());
        }
        // 返回的 url 为 http://，统一转 https 避免混合内容
        Ok(url.replace("http://", "https://"))
    }

    async fn get_lyrics(&self, track_id: &str) -> Result<Option<String>, String> {
        // 歌词接口匿名可用，返回明文 LRC
        let resp = self
            .client
            .get(format!(
                "{}/api/song/lyric?id={}&lv=-1&kv=-1&tv=-1&csrf_token=",
                API_BASE, track_id
            ))
            .headers(self.base_headers())
            .send()
            .await
            .map_err(|e| format!("获取歌词失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析歌词失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            return Ok(None);
        }
        let lyric = j["lrc"]["lyric"].as_str().unwrap_or("");
        if lyric.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lyric.to_string()))
        }
    }

    async fn get_user_playlists(&self) -> Result<Vec<Playlist>, String> {
        self.fetch_user_playlists().await
    }

    async fn get_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String> {
        self.fetch_playlist_detail(playlist_id).await
    }
}
