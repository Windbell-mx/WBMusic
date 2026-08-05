/**
 * Tauri 后端调用封装
 *
 * 前端所有与 Rust 后端交互的入口，统一走 @tauri-apps/api 的 invoke。
 * 在浏览器环境（纯前端调试时）自动降级为 mock，保证开发体验。
 */

import { invoke } from '@tauri-apps/api/core'

/** 音乐源类型 */
export type MusicSource = 'qq_music' | 'netease'

/** 歌曲信息 */
export interface Track {
  id: string
  title: string
  artist: string
  album?: string | null
  duration?: number | null
  cover_url?: string | null
  source: MusicSource
}

/** 搜索结果 */
export interface SearchResult {
  tracks: Track[]
  total: number
}

/** 登录状态 */
export interface LoginStatus {
  logged_in: boolean
  nickname?: string | null
  user_id?: string | null
}

/** 登录凭据 */
export type LoginCredential =
  | { type: 'cookie'; value: string }
  | { type: 'token'; value: string }
  | { type: 'password'; username: string; password: string }

/** 是否运行在 Tauri 环境 */
export const isTauri = '__TAURI_INTERNALS__' in window

/**
 * 搜索音乐
 */
export async function searchMusic(
  keyword: string,
  source: MusicSource,
  limit = 30,
): Promise<SearchResult> {
  if (isTauri) {
    return invoke<SearchResult>('search_music', { keyword, source, limit })
  }
  // 浏览器降级：返回 mock 数据
  return {
    tracks: [
      {
        id: `${source}-mock-${Date.now()}`,
        title: `${keyword}（演示）`,
        artist: '演示歌手',
        duration: 240,
        source,
      },
    ],
    total: 1,
  }
}

/**
 * 登录音乐源
 */
export async function loginMusic(
  source: MusicSource,
  credential: LoginCredential,
): Promise<LoginStatus> {
  if (isTauri) {
    return invoke<LoginStatus>('login_music', { source, credential })
  }
  // 浏览器降级：直接返回成功
  return { logged_in: true, nickname: '演示用户' }
}

/**
 * 退出登录音乐源
 */
export async function logoutMusic(source: MusicSource): Promise<void> {
  if (isTauri) {
    return invoke('logout_music', { source })
  }
}

/**
 * 打开扫码登录窗口（真实浏览器环境）
 *
 * QQ 音乐窗口内支持 QQ / 微信两种扫码方式；
 * 登录成功后后端会自动完成登录并触发 `qr-login-success` 事件。
 */
export async function openQrLogin(source: MusicSource): Promise<void> {
  if (isTauri) {
    return invoke('open_qr_login', { source })
  }
}

/**
 * 查询所有音乐源登录状态
 */
export async function getLoginStatus(): Promise<Record<MusicSource, LoginStatus>> {
  if (isTauri) {
    return invoke('get_login_status')
  }
  // 浏览器降级
  return {
    qq_music: { logged_in: false },
    netease: { logged_in: false },
  }
}

/**
 * 获取歌曲播放地址
 */
export async function getTrackUrl(source: MusicSource, trackId: string): Promise<string> {
  if (isTauri) {
    return invoke<string>('get_track_url', { source, trackId })
  }
  // 浏览器降级：返回一个示例音频
  return 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3'
}

/**
 * 获取应用缓存目录（完整绝对路径）
 */
export async function getCacheDir(): Promise<string> {
  if (isTauri) {
    return invoke<string>('get_cache_dir')
  }
  // 浏览器降级
  return 'WBMusic/cache'
}

/**
 * 获取歌词
 */
export async function getLyrics(source: MusicSource, trackId: string): Promise<string | null> {
  if (isTauri) {
    return invoke<string | null>('get_lyrics', { source, trackId })
  }
  return null
}

/** 用户歌单 */
export interface Playlist {
  id: string
  name: string
  description?: string | null
  cover_url?: string | null
  track_count: number
  play_count: number
  source: MusicSource
}

/** 歌单详情（含歌曲列表） */
export interface PlaylistDetail {
  id: string
  name: string
  description?: string | null
  cover_url?: string | null
  track_count: number
  tracks: Track[]
}

/**
 * 获取当前登录用户的歌单列表
 */
export async function getUserPlaylists(source: MusicSource): Promise<Playlist[]> {
  if (isTauri) {
    return invoke<Playlist[]>('get_user_playlists', { source })
  }
  // 浏览器降级：返回空
  return []
}

/**
 * 获取歌单详情（含歌曲列表）
 */
export async function getPlaylistDetail(
  source: MusicSource,
  playlistId: string,
): Promise<PlaylistDetail> {
  if (isTauri) {
    return invoke<PlaylistDetail>('get_playlist_detail', { source, playlistId })
  }
  // 浏览器降级
  return {
    id: playlistId,
    name: '歌单',
    description: null,
    cover_url: null,
    track_count: 0,
    tracks: [],
  }
}
