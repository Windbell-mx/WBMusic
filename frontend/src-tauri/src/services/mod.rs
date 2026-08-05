//! 业务服务层
//!
//! 定义第三方音乐源接入的统一抽象接口（trait），
//! 并分别实现 QQ 音乐、网易云音乐等适配器。
//!
//! 与纯 Web 后端的区别：此处服务运行在 Tauri 桌面应用内部，
//! 通过 `#[tauri::command]` 暴露给前端调用，天然无跨域问题。

pub mod music_provider;
pub mod netease;
pub mod qq_music;
pub mod session;

pub use music_provider::{
    LoginCredential, LoginStatus, MusicProvider, MusicProviderKind, Playlist, PlaylistDetail,
    SearchResult,
};
pub use netease::NeteaseProvider;
pub use qq_music::QqMusicProvider;
