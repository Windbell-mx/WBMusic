//! QQ 音乐接入服务
//!
//! 通过 Cookie（浏览器登录 y.qq.com 后复制）调用 QQ 音乐官方接口：
//!   - 搜索：POST https://u.y.qq.com/cgi-bin/musicu.fcg（无需登录）
//!   - 播放地址：POST musicu.fcg 模块 UrlGetVkey（登录后可播 VIP）
//!   - 歌词：GET https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg

use std::collections::HashMap;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::qq_enc;

    fn anon_provider() -> QqMusicProvider {
        QqMusicProvider::new(ProviderSession {
            logged_in: false,
            nickname: None,
            user_id: None,
            credential: None,
        })
    }

    /// 排行榜新接口全链路：GetDetail + songId->songmid 批量转换
    #[tokio::test]
    async fn toplist_detail_works() {
        let p = anon_provider();
        // topid=26 热歌榜（实测 totalNum=284）
        let d = p
            .get_playlist_detail("qq:toplist:26")
            .await
            .expect("获取排行榜失败");
        println!("榜单: {} | 歌曲数: {}", d.name, d.tracks.len());
        assert!(d.tracks.len() >= 200, "歌曲数应>=200, 实际 {}", d.tracks.len());
        for t in d.tracks.iter().take(5) {
            println!("  {} - {} (id={})", t.title, t.artist, t.id);
            assert!(!t.id.is_empty(), "songmid 不应为空");
            assert!(!t.title.is_empty(), "标题不应为空");
        }
    }

    /// 排行榜歌曲可播放（UrlGetVkey 用 songmid 拿到 purl）
    #[tokio::test]
    async fn toplist_track_playable() {
        let p = anon_provider();
        let d = p
            .get_playlist_detail("qq:toplist:26")
            .await
            .expect("获取排行榜失败");
        let first = &d.tracks[0];
        let url = p.get_track_url(&first.id).await.expect("获取播放地址失败");
        println!("播放地址: {}", url);
        assert!(!url.is_empty(), "播放地址不应为空");
    }

    /// 验证 uniform_get_Dissinfo 分页全链路：477 首歌单应全量返回（登录态）
    #[tokio::test]
    async fn dissinfo_pagination_capacity() {
        // 凭据从环境变量读取（测试账号），未设置时跳过
        let cred = match std::env::var("QQ_TEST_CREDENTIAL") {
            Ok(c) if !c.is_empty() => c,
            _ => {
                eprintln!("跳过：未设置 QQ_TEST_CREDENTIAL（格式 uin=xxx; qm_keyst=xxx）");
                return;
            }
        };
        let p = QqMusicProvider::new(ProviderSession {
            logged_in: true,
            nickname: None,
            user_id: None,
            credential: Some(cred.clone()),
        });
        // 该测试歌单含 477 首（昨日观影纪录片喜欢），验证分页不会截断
        let d = p
            .get_playlist_detail("9541850155")
            .await
            .expect("获取歌单失败");
        println!("歌单: {} | 歌曲数: {}", d.name, d.tracks.len());
        assert!(d.tracks.len() >= 400, "歌曲数应>=400(全量), 实际 {}", d.tracks.len());
        for t in d.tracks.iter().take(3) {
            println!("  {} - {} (id={})", t.title, t.artist, t.id);
        }
    }

    /// 验证收藏/取消收藏（musics.fcg 加密链路）
    #[tokio::test]
    async fn like_track_works() {
        // 凭据从环境变量读取（测试账号），未设置时跳过
        let cred = match std::env::var("QQ_TEST_CREDENTIAL") {
            Ok(c) if !c.is_empty() => c,
            _ => {
                eprintln!("跳过：未设置 QQ_TEST_CREDENTIAL（格式 uin=xxx; qm_keyst=xxx）");
                return;
            }
        };
        let p = QqMusicProvider::new(ProviderSession {
            logged_in: true,
            nickname: None,
            user_id: None,
            credential: Some(cred.clone()),
        });
        // 用排行榜热歌（晴天-周杰伦 002n0SId3TkNwS 是杨宗纬；随便选一首已验证可播的）
        let d = p
            .get_playlist_detail("qq:toplist:26")
            .await
            .expect("获取排行榜失败");
        let tid = d.tracks[0].id.clone();
        println!("测试收藏: {}", tid);
        // 直接复现 Add 请求
        let (uin, key) = p.session_cred().unwrap();
        let gtk = qq_enc::g_tk(&key);
        let mk_body = |method: &str| {
            json!({
                "comm": {
                    "cv": 4747474, "ct": 24, "format": "json", "inCharset": "utf-8",
                    "outCharset": "utf-8", "notice": 0, "platform": "yqq.json",
                    "needNewCode": 1, "uin": uin.parse::<i64>().unwrap_or(0),
                    "g_tk_new_20200303": gtk, "g_tk": gtk,
                },
                "req_1": {
                    "module": "music.musicasset.PlaylistDetailWrite",
                    "method": method,
                    "param": {
                        "dirId": 201,
                        "v_songInfo": [{ "songType": 0, "songMid": tid }]
                    }
                }
            })
        };
        // Add 请求
        let (sign_a, cipher_a) = {
            let json_str = mk_body("AddSonglist").to_string();
            tokio::task::spawn_blocking(move || {
                let sign = qq_enc::sign(&json_str)?;
                let cipher = qq_enc::encrypt(&json_str)?;
                Ok::<(String, String), String>((sign, cipher))
            })
            .await
            .unwrap()
            .unwrap()
        };
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let url = |sign: &str| format!("https://u6.y.qq.com/cgi-bin/musics.fcg?_={}&encoding=ag-1&sign={}", ts, sign);
        let cookie = format!("uin={}; qqmusic_key={}; qm_keyst={}", uin, key, key);
        let do_post = |url: String, cipher: String| {
            let client = p.client.clone();
            let cookie = cookie.clone();
            async move {
                client.post(&url)
                    .header("Content-Type", "text/plain")
                    .header("Referer", "https://y.qq.com/")
                    .header("Origin", "https://y.qq.com")
                    .header("Cookie", cookie)
                    .header("Accept-Encoding", "identity")
                    .body(cipher)
                    .send().await.unwrap().bytes().await.unwrap()
            }
        };
        let add_bytes = do_post(url(&sign_a), cipher_a).await;
        std::fs::write(r"C:\Users\windbell02\AppData\Local\Temp\wbmusic_js\resp_add.bin", &add_bytes).ok();
        println!("Add响应长度: {} 字节", add_bytes.len());
        // Add 解密
        let add_plain = tokio::task::spawn_blocking(move || qq_enc::decrypt(&add_bytes)).await.unwrap();
        println!("Add解密结果: {:?}", add_plain);
        // Del 请求
        let (sign_d, cipher_d) = {
            let json_str = mk_body("DelSonglist").to_string();
            tokio::task::spawn_blocking(move || {
                let sign = qq_enc::sign(&json_str)?;
                let cipher = qq_enc::encrypt(&json_str)?;
                Ok::<(String, String), String>((sign, cipher))
            })
            .await
            .unwrap()
            .unwrap()
        };
        let del_bytes = do_post(url(&sign_d), cipher_d).await;
        std::fs::write(r"C:\Users\windbell02\AppData\Local\Temp\wbmusic_js\resp_del.bin", &del_bytes).ok();
        println!("Del响应长度: {} 字节", del_bytes.len());
        let del_plain = tokio::task::spawn_blocking(move || qq_enc::decrypt(&del_bytes)).await.unwrap();
        println!("Del解密结果: {:?}", del_plain);
    }
}

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
    ///
    /// 使用新版 PC 端歌单详情接口（musicu.fcg / music.srfDissInfo.aiDissInfo）：
    /// - 旧接口 fcg_ucc_getcdinfo_byids_cp.fcg 已失效：扫码登录（psrf_* token）拿不到
    ///   旧版 skey/p_skey cookie，导致该接口无论是否登录都返回 `check privacy error`，
    ///   前端误判为"需要登录"。
    /// - 新接口匿名即可返回完整歌单信息与歌曲列表，无需登录。
    async fn fetch_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String> {
        // 排行榜详情（id 格式 qq:toplist:<topid>）
        if let Some(topid) = playlist_id.strip_prefix("qq:toplist:") {
            return self.fetch_toplist_detail(topid).await;
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/"),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        // 分页拉取全部歌曲：每次 song_num 首，直到不足一页或达到总数
        let mut dirinfo: Value = Value::Null;
        let mut tracks: Vec<Track> = Vec::new();
        let mut begin: i64 = 0;
        const PAGE: i64 = 200;
        loop {
            let body = json!({
                "comm": {
                    "cv": 13020508,
                    "ct": 24,
                    "format": "json",
                    "inCharset": "utf-8",
                    "outCharset": "utf-8",
                    "notice": 0,
                    "platform": "yqq.json",
                    "needNewCode": 1,
                    "uin": "0",
                    "g_tk": 5381,
                },
                "req_0": {
                    "module": "music.srfDissInfo.aiDissInfo",
                    "method": "uniform_get_Dissinfo",
                    "param": {
                        "disstid": playlist_id.parse::<i64>().unwrap_or(0),
                        "enc_host_uin": "",
                        "tag": 1,
                        "userinfo": 1,
                        "song_begin": begin,
                        "song_num": PAGE,
                    },
                },
            });
            let resp = self
                .client
                .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("获取歌单详情失败: {}", e))?;
            let j: Value = resp
                .json()
                .await
                .map_err(|e| format!("解析歌单详情失败: {}", e))?;
            let info = &j["req_0"]["data"];
            let code = info["subcode"].as_i64().unwrap_or(-1);
            if code != 0 {
                let msg = info["msg"].as_str().unwrap_or("未知错误");
                return Err(format!("获取歌单详情失败: {}", msg));
            }
            if dirinfo.is_null() {
                dirinfo = info["dirinfo"].clone();
            }
            let total = info["total_song_num"].as_u64();
            let songs = info["songlist"].as_array().cloned().unwrap_or_default();
            let batch: Vec<Track> = songs
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
                    let album = s["album"]["name"].as_str().map(|x| x.to_string());
                    let duration = s["interval"].as_u64().map(|d| d as u32);
                    let album_mid = s["album"]["mid"].as_str().unwrap_or("");
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
            tracks.extend(batch);
            // 判断是否还有下一页
            let fetched = songs.len() as i64;
            if fetched < PAGE {
                break;
            }
            begin += fetched;
            if let Some(t) = total {
                if tracks.len() as u64 >= t {
                    break;
                }
            }
        }
        let cover_url = dirinfo["picurl"]
            .as_str()
            .map(|s| s.replace("http://", "https://"));
        Ok(PlaylistDetail {
            id: playlist_id.to_string(),
            name: dirinfo["title"].as_str().unwrap_or("未命名歌单").to_string(),
            description: dirinfo["desc"].as_str().map(|s| s.to_string()),
            cover_url,
            track_count: tracks.len() as u32,
            tracks,
        })
    }

    /// 获取排行榜详情（匿名可用，无需登录）
    ///
    /// 旧接口 `fcg_v8_toplist_cp.fcg?topid=<id>` 已失效（返回 code=-1），
    /// 改用官网新版接口 `musicToplist.ToplistInfoServer / GetDetail`（musicu.fcg）：
    /// - 第一步：GetDetail 分页拉取榜单全部歌曲（字段：songId/albumMid/title/singerName）
    /// - 第二步：批量用 `music.pf_song_detail_svr / get_song_detail_yqq` 把数字 songId
    ///   转换为 songmid（播放/歌词接口需要 songmid）
    async fn fetch_toplist_detail(&self, topid: &str) -> Result<PlaylistDetail, String> {
        let topid_num = topid.parse::<i64>().unwrap_or(0);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/"),
        );
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        // ---------- 第一步：GetDetail 分页拉全量 ----------
        let mut raw_songs: Vec<Value> = Vec::new();
        let mut meta: Option<Value> = None;
        let mut offset: i64 = 0;
        const PAGE: i64 = 300;
        loop {
            let body = json!({
                "comm": {
                    "cv": 4747474,
                    "ct": 24,
                    "format": "json",
                    "inCharset": "utf-8",
                    "outCharset": "utf-8",
                    "notice": 0,
                    "platform": "yqq.json",
                    "needNewCode": 1,
                    "uin": "0",
                    "g_tk": 5381,
                },
                "req_0": {
                    "module": "musicToplist.ToplistInfoServer",
                    "method": "GetDetail",
                    "param": {
                        "topid": topid_num,
                        "offset": offset,
                        "num": PAGE,
                        "period": "",
                    },
                },
            });
            let resp = self
                .client
                .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("获取排行榜详情失败: {}", e))?;
            let j: Value = resp
                .json()
                .await
                .map_err(|e| format!("解析排行榜详情失败: {}", e))?;
            let code = j["req_0"]["code"].as_i64().unwrap_or(-1);
            if code != 0 {
                return Err(format!("获取排行榜详情失败 (code={})", code));
            }
            let d = &j["req_0"]["data"]["data"];
            if d.is_null() {
                return Err("排行榜详情响应格式异常".to_string());
            }
            if meta.is_none() {
                meta = Some(d.clone());
            }
            let songs = d["song"].as_array().cloned().unwrap_or_default();
            let n = songs.len() as i64;
            raw_songs.extend(songs);
            if n == 0 || n < PAGE {
                break;
            }
            offset += n;
        }
        let meta = meta.ok_or("排行榜详情响应格式异常")?;
        // ---------- 第二步：批量 songId -> songmid ----------
        let song_ids: Vec<i64> = raw_songs
            .iter()
            .filter_map(|s| s["songId"].as_i64())
            .collect();
        let mut mid_map: HashMap<i64, Value> = HashMap::new();
        // 注意：musicu.fcg 单请求内 req 块过多会返回 code=500000（实测 50 个失败、20 个成功）
        for chunk in song_ids.chunks(20) {
            let mut body = json!({
                "comm": {
                    "cv": 4747474,
                    "ct": 24,
                    "format": "json",
                    "inCharset": "utf-8",
                    "outCharset": "utf-8",
                    "notice": 0,
                    "platform": "yqq.json",
                    "needNewCode": 1,
                    "uin": "0",
                    "g_tk": 5381,
                }
            });
            for (i, id) in chunk.iter().enumerate() {
                body[format!("req_{}", i)] = json!({
                    "module": "music.pf_song_detail_svr",
                    "method": "get_song_detail_yqq",
                    "param": { "song_id": *id, "song_type": 0 },
                });
            }
            let resp = self
                .client
                .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("获取歌曲信息失败: {}", e))?;
            let j: Value = resp
                .json()
                .await
                .map_err(|e| format!("解析歌曲信息失败: {}", e))?;
            for (i, id) in chunk.iter().enumerate() {
                let ti = &j[format!("req_{}", i)]["data"]["track_info"];
                if !ti.is_null() {
                    mid_map.insert(*id, ti.clone());
                }
            }
        }
        // ---------- 第三步：组装歌曲列表 ----------
        let tracks: Vec<Track> = raw_songs
            .iter()
            .filter_map(|s| {
                let sid = s["songId"].as_i64().unwrap_or(0);
                let ti = mid_map.get(&sid);
                let mid = ti
                    .and_then(|t| t["mid"].as_str())
                    .unwrap_or("")
                    .to_string();
                if mid.is_empty() {
                    return None; // 转不到 songmid 的歌曲跳过
                }
                let title = ti
                    .and_then(|t| t["name"].as_str())
                    .or_else(|| s["title"].as_str())
                    .unwrap_or("未知歌曲")
                    .to_string();
                let artist = ti
                    .and_then(|t| t["singer"].as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x["name"].as_str())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .or_else(|| s["singerName"].as_str().map(|x| x.to_string()))
                    .unwrap_or_default();
                let album = ti
                    .and_then(|t| t["album"]["name"].as_str())
                    .map(|x| x.to_string());
                let duration = ti
                    .and_then(|t| t["interval"].as_u64())
                    .map(|x| x as u32);
                let album_mid = ti
                    .and_then(|t| t["album"]["mid"].as_str())
                    .or_else(|| s["albumMid"].as_str())
                    .unwrap_or("");
                let cover_url = if album_mid.is_empty() {
                    None
                } else {
                    Some(format!(
                        "https://y.gtimg.cn/music/photo_new/T002R800x800M000{}.jpg",
                        album_mid
                    ))
                };
                Some(Track {
                    id: mid,
                    title,
                    artist,
                    album,
                    duration,
                    cover_url,
                    source: MusicProviderKind::QqMusic,
                })
            })
            .collect();
        let cover_url = meta["pic"]
            .as_str()
            .map(|s| s.replace("http://", "https://"))
            .or_else(|| {
                meta["cover"]
                    .as_str()
                    .map(|s| s.replace("http://", "https://"))
            });
        let playlist_id = format!("qq:toplist:{}", topid);
        Ok(PlaylistDetail {
            id: playlist_id,
            name: meta["title"].as_str().unwrap_or("未命名榜单").to_string(),
            description: meta["intro"].as_str().map(|s| s.to_string()),
            cover_url,
            track_count: tracks.len() as u32,
            tracks,
        })
    }

    /// 获取热门歌单（匿名可用，无需登录）
    ///
    /// 使用 PC 端歌单广场接口 `fcg_get_diss_by_tag.fcg`，
    /// `categoryId=10000000`（全部分类）+ `sortId=5`（按播放量排序）。
    async fn fetch_recommended_playlists(&self, limit: u32) -> Result<Vec<Playlist>, String> {
        self.fetch_playlists_by_sort(5, limit).await
    }

    /// 通过歌单广场接口按指定排序方式获取歌单（匿名可用，无需登录）
    ///
    /// - `sortId=5`：按播放量排序（热门）
    /// - `sortId=1`：推荐排序
    async fn fetch_playlists_by_sort(&self, sort_id: i64, limit: u32) -> Result<Vec<Playlist>, String> {
        let limit = limit.clamp(1, 50);
        let ein = (limit - 1) as i64;
        let url = format!(
            "https://c.y.qq.com/splcloud/fcgi-bin/fcg_get_diss_by_tag.fcg?\
             inCharset=utf8&outCharset=utf-8&format=json&platform=yqq&needNewCode=0&\
             new_format=1&picmid=1&categoryId=10000000&sortId={}&sin=0&ein={}",
            sort_id, ein
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/"),
        );
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("获取热门歌单失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析热门歌单失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            return Err(format!("获取热门歌单失败 (code={})", code));
        }
        let list = j["data"]["list"]
            .as_array()
            .ok_or("热门歌单响应格式异常")?;
        let result = list
            .iter()
            .map(|p| {
                // 注意：dissid 是字符串（如 "7707261125"），不能只用 as_i64()，
                // 否则解析失败回退为 "0" 导致所有歌单 id 重复
                let id = p["dissid"]
                    .as_str()
                    .map(|s| s.to_string())
                    .or_else(|| p["dissid"].as_i64().map(|v| v.to_string()))
                    .or_else(|| p["tid"].as_str().map(|s| s.to_string()))
                    .or_else(|| p["tid"].as_i64().map(|v| v.to_string()))
                    .unwrap_or_default();
                // imgurl 为 http:// 开头，统一转 https:// 避免混合内容
                let cover_url = p["imgurl"].as_str().map(|s| {
                    s.replace("http://", "https://")
                });
                Playlist {
                    id,
                    name: p["dissname"].as_str().unwrap_or("未命名歌单").to_string(),
                    // 注意：该接口描述字段是 introduction（不是 diss_desc）
                    description: p["introduction"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| p["diss_desc"].as_str().map(|s| s.to_string())),
                    cover_url,
                    track_count: p["song_cnt"].as_u64().unwrap_or(0) as u32,
                    play_count: p["listen_num"].as_u64().unwrap_or(0),
                    source: MusicProviderKind::QqMusic,
                }
            })
            .collect();
        Ok(result)
    }

    /// 获取首页分类歌单
    ///
    /// - `rec`：推荐歌单（编辑推荐排序）
    /// - `hot`：排行榜（官方榜单）
    async fn fetch_category_playlists(&self, category: &str, limit: u32) -> Result<Vec<Playlist>, String> {
        match category {
            "rec" => self.fetch_playlists_by_sort(1, limit).await,
            "hot" => self.fetch_hot_charts(limit).await,
            _ => Err(format!("不支持的分类: {}", category)),
        }
    }

    /// 获取官方排行榜（匿名可用，无需登录）
    ///
    /// 使用榜单列表接口 `fcg_myqq_toplist.fcg`，返回巅峰榜等官方榜单。
    /// 榜单详情可用 `fcg_v8_toplist_cp.fcg?topid=<id>` 获取。
    async fn fetch_hot_charts(&self, limit: u32) -> Result<Vec<Playlist>, String> {
        let url = "https://c.y.qq.com/v8/fcg-bin/fcg_myqq_toplist.fcg?format=json";
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://y.qq.com/"),
        );
        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("获取排行榜失败: {}", e))?;
        let j: Value = resp
            .json()
            .await
            .map_err(|e| format!("解析排行榜失败: {}", e))?;
        let code = j["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            return Err(format!("获取排行榜失败 (code={})", code));
        }
        let list = j["data"]["topList"]
            .as_array()
            .ok_or("排行榜响应格式异常")?;
        let limit = limit as usize;
        let result = list
            .iter()
            .take(limit)
            .map(|p| {
                let cover_url = p["picUrl"]
                    .as_str()
                    .map(|s| s.replace("http://", "https://"));
                Playlist {
                    // 排行榜 id 用 topid 前缀，详情走 fcg_v8_toplist_cp.fcg?topid=
                    id: format!("qq:toplist:{}", p["id"].as_i64().unwrap_or(0)),
                    name: p["topTitle"].as_str().unwrap_or("未命名榜单").to_string(),
                    description: Some("QQ 音乐官方排行榜".to_string()),
                    cover_url,
                    track_count: 0,
                    play_count: p["listenCount"].as_u64().unwrap_or(0),
                    source: MusicProviderKind::QqMusic,
                }
            })
            .collect();
        Ok(result)
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

    async fn get_recommended_playlists(&self, limit: u32) -> Result<Vec<Playlist>, String> {
        self.fetch_recommended_playlists(limit).await
    }

    async fn get_category_playlists(&self, category: &str, limit: u32) -> Result<Vec<Playlist>, String> {
        self.fetch_category_playlists(category, limit).await
    }

    async fn like_track(&self, track_id: &str, like: bool) -> Result<(), String> {
        // 需登录（解析到 uin + key），未登录直接报错
        let (uin, key) = self.session_cred()?;
        if key.is_empty() {
            return Err("请先登录 QQ 音乐".into());
        }
        // 腾讯 musics.fcg 接口：明文 JSON 需先 AES-GCM 加密（__cgiEncrypt），
        // 请求带 sign 参数，响应体再解密（__cgiDecrypt）。
        // 加解密引擎（rquickjs + Rust AES-GCM）为阻塞调用，放到阻塞线程池执行。
        let gtk = super::qq_enc::g_tk(&key);
        let body = json!({
            "comm": {
                "cv": 4747474,
                "ct": 24,
                "format": "json",
                "inCharset": "utf-8",
                "outCharset": "utf-8",
                "notice": 0,
                "platform": "yqq.json",
                "needNewCode": 1,
                "uin": uin.parse::<i64>().unwrap_or(0),
                "g_tk_new_20200303": gtk,
                "g_tk": gtk,
            },
            "req_1": {
                "module": "music.musicasset.PlaylistDetailWrite",
                "method": if like { "AddSonglist" } else { "DelSonglist" },
                "param": {
                    "dirId": 201,
                    "v_songInfo": [{
                        "songType": 0,
                        "songMid": track_id,
                    }]
                }
            }
        });
        let json_str = body.to_string();

        // sign + encrypt 在阻塞线程中执行（内部走专用引擎线程）
        let (sign, cipher) = tokio::task::spawn_blocking(move || {
            let sign = super::qq_enc::sign(&json_str)?;
            let cipher = super::qq_enc::encrypt(&json_str)?;
            Ok::<(String, String), String>((sign, cipher))
        })
        .await
        .map_err(|e| format!("收藏加密线程失败: {}", e))??;

        // POST 加密请求到 musics.fcg
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let url = format!(
            "https://u6.y.qq.com/cgi-bin/musics.fcg?_={}&encoding=ag-1&sign={}",
            ts, sign
        );
        let cookie = format!("uin={}; qqmusic_key={}; qm_keyst={}", uin, key, key);
        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "text/plain")
            .header("Referer", "https://y.qq.com/")
            .header("Origin", "https://y.qq.com")
            .header("Cookie", cookie)
            .header("Accept-Encoding", "identity")
            .body(cipher)
            .send()
            .await
            .map_err(|e| format!("收藏请求失败: {}", e))?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("读取收藏响应失败: {}", e))?;

        // 解密响应体
        let plain = tokio::task::spawn_blocking(move || super::qq_enc::decrypt(&bytes))
            .await
            .map_err(|e| format!("收藏解密线程失败: {}", e))??;

        // 解析解密后的 JSON，检查业务 code
        let j: Value = serde_json::from_str(&plain)
            .map_err(|e| format!("解析收藏响应失败: {}", e))?;
        let code = j["req_1"]["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            return Err(format!(
                "收藏失败 (code={})：登录态可能已失效，请重新登录 QQ 音乐",
                code
            ));
        }
        Ok(())
    }
}
