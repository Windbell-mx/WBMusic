# WBMusic 音乐播放器

基于 Tauri v2 的桌面音乐播放器：Vue 3 前端 + Rust 后端，打包为 Windows 安装程序（NSIS）。

## 项目结构

```
wbmusic/
└── frontend/        # 前端：Vue 3 + TypeScript + Vite + Naive UI
    └── src-tauri/   # Tauri 桌面壳：Rust 服务层 + 打包配置
```

## 开发

```bash
cd frontend
npm install

# 纯前端开发（浏览器调试，API 自动降级为 mock）
npm run dev          # http://localhost:5173

# Tauri 桌面开发（打开桌面窗口）
npm run tauri:dev
```

## 打包安装程序

```bash
cd frontend
npm run tauri:build
```

产物位于 `frontend/src-tauri/target/release/bundle/nsis/`，生成
`WBMusic_0.1.0_x64-setup.exe` 安装程序。

## 架构说明

- **前端**：Vue 3 + Naive UI + Pinia，通过 `@tauri-apps/api` 的 `invoke()` 调用 Rust 命令
  - `stores/player.ts` — 全局播放状态（唯一 Audio 实例，底部 PlayerBar 与全屏 PlayerView 联动）
  - `api/index.ts` — Tauri invoke 封装，浏览器环境自动降级为 mock
- **Rust 服务层**（`src-tauri/src/services/`）：
  - `music_provider.rs` — 音乐源统一抽象 trait
  - `qq_music.rs` / `netease.rs` — QQ 音乐、网易云适配器（Cookie 登录）
  - `session.rs` — 登录会话持久化（保存到 `%APPDATA%/com.wbmusic.app/session.json`）
- **命令层**（`src-tauri/src/commands.rs`）：`search_music` / `login_music` / `logout_music` /
  `get_login_status` / `get_track_url` / `get_lyrics`

## 登录方式

当前支持 Cookie 登录：在浏览器登录 QQ 音乐 / 网易云后，将请求中的
Cookie 粘贴到「设置 → 账号与音乐源」即可。登录态保存在本机，重启自动恢复。

> 注意：第三方音乐源私有接口存在法律与合规风险，请确保接入方式符合相关条款。

