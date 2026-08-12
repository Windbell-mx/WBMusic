//! 第三方音乐源统一抽象接口
//!
//! 通过 `MusicProvider` trait 定义所有音乐源需要实现的统一能力，
//! 上层业务只依赖该 trait，不关心具体是哪个音乐源。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 音乐源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicProviderKind {
    /// QQ 音乐
    QqMusic,
    /// 网易云音乐
    Netease,
}

impl MusicProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MusicProviderKind::QqMusic => "qq_music",
            MusicProviderKind::Netease => "netease",
        }
    }

    /// 从字符串解析，未知值返回 None
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "qq_music" => Some(MusicProviderKind::QqMusic),
            "netease" => Some(MusicProviderKind::Netease),
            _ => None,
        }
    }
}

/// 歌曲信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<u32>,
    pub cover_url: Option<String>,
    #[serde(rename = "source")]
    pub source: MusicProviderKind,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub tracks: Vec<Track>,
    pub total: usize,
}

/// 用户歌单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub track_count: u32,
    pub play_count: u64,
    pub source: MusicProviderKind,
}

/// 歌单详情（含歌曲列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub track_count: u32,
    pub tracks: Vec<Track>,
}

/// 登录凭据（账号密码 / Cookie / Token）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum LoginCredential {
    /// 账号密码登录
    Password { username: String, password: String },
    /// 直接提供 Cookie（从浏览器抓取）
    Cookie(String),
    /// Token 登录
    Token(String),
}

/// 登录状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginStatus {
    pub logged_in: bool,
    pub nickname: Option<String>,
    pub user_id: Option<String>,
}

/// 音乐源提供者统一接口
///
/// 所有第三方音乐接入（QQ 音乐、网易云等）都实现该 trait，
/// 便于上层服务统一调用与切换。
#[async_trait]
pub trait MusicProvider: Send + Sync {
    /// 返回该提供者的类型
    fn kind(&self) -> MusicProviderKind;

    /// 登录。成功返回 true。
    async fn login(&self, credential: &LoginCredential) -> Result<bool, String>;

    /// 退出登录
    async fn logout(&self) -> Result<(), String>;

    /// 获取当前登录用户信息
    fn login_status(&self) -> LoginStatus;

    /// 校验登录态是否仍然有效。
    ///
    /// 返回 `Ok(true)` 表示凭据仍有效；`Ok(false)` 表示已失效（登录态过期/
    /// 凭据被吊销等），调用方应触发自动登出。实现方应尽量做真实的网络校验
    /// （如调用需要登录态的接口），而非仅检查内存标志。
    async fn validate_login(&self) -> Result<bool, String>;

    /// 搜索歌曲
    async fn search(&self, keyword: &str, limit: u32) -> Result<SearchResult, String>;

    /// 获取歌曲播放地址（URL）
    async fn get_track_url(&self, track_id: &str) -> Result<String, String>;

    /// 获取歌词
    async fn get_lyrics(&self, track_id: &str) -> Result<Option<String>, String>;

    /// 获取当前登录用户的歌单列表
    async fn get_user_playlists(&self) -> Result<Vec<Playlist>, String>;

    /// 获取歌单详情（含歌曲列表）
    async fn get_playlist_detail(&self, playlist_id: &str) -> Result<PlaylistDetail, String>;

    /// 获取推荐歌单（热门/精品歌单，匿名可用，无需登录）
    async fn get_recommended_playlists(&self, limit: u32) -> Result<Vec<Playlist>, String>;

    /// 获取首页分类歌单（每日推荐/精选/热歌榜等，按平台实现各自支持的分类）
    async fn get_category_playlists(&self, category: &str, limit: u32) -> Result<Vec<Playlist>, String>;

    /// 收藏/取消收藏歌曲（红心）。`like=true` 收藏到平台默认喜欢歌单，`false` 取消
    async fn like_track(&self, track_id: &str, like: bool) -> Result<(), String>;
}
