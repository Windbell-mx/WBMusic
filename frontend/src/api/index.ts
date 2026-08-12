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
 * 全局刷新：后端校验所有已登录源的登录态（失效的自动登出），
 * 返回各源最新登录状态。前端在刷新按钮 / F5 时调用。
 */
export async function refreshAll(): Promise<Record<MusicSource, LoginStatus>> {
  if (isTauri) {
    return invoke('refresh_all')
  }
  return getLoginStatus()
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
 * 获取应用缓存目录（完整绝对路径），默认位于安装位置下
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

/**
 * 收藏/取消收藏歌曲（红心）
 * @param source 音乐源
 * @param trackId 歌曲 ID（网易云为数字 ID，QQ 为 songmid）
 * @param like true=收藏（加入默认喜欢歌单），false=取消
 */
export async function likeTrack(
  source: MusicSource,
  trackId: string,
  like: boolean,
): Promise<void> {
  if (isTauri) {
    return invoke('like_track', { source, trackId, like })
  }
  // 浏览器降级：直接成功
  return
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

/**
 * 获取推荐歌单（热门/精品歌单，匿名可用，无需登录）
 */
export async function getRecommendedPlaylists(
  source: MusicSource,
  limit = 12,
): Promise<Playlist[]> {
  if (isTauri) {
    return invoke<Playlist[]>('get_recommended_playlists', { source, limit })
  }
  // 浏览器降级：返回几条演示数据
  const mockNames =
    source === 'netease'
      ? ['每日推荐', '华语流行', '欧美金曲', '轻音乐精选', 'ACG 音乐']
      : ['QQ 热歌榜', '飙升榜', '说唱新势力', '民谣之声', '电影原声带']
  return mockNames.map((name, i) => ({
    id: `${source}-recommend-${i}`,
    name,
    description: '推荐歌单（演示）',
    cover_url: `https://picsum.photos/seed/${source}-rec-${i}/400/400`,
    track_count: 50 + i * 10,
    play_count: 100000 * (i + 1),
    source,
  }))
}

/** 首页分类标识 */
export type HomeCategory = 'featured' | 'hot' | 'daily' | 'rec'

/**
 * 获取首页分类歌单
 *
 * 网易云：featured=精选、hot=热歌榜、daily=每日推荐
 * QQ 音乐：rec=推荐、hot=排行榜
 */
export async function getCategoryPlaylists(
  source: MusicSource,
  category: HomeCategory,
  limit = 10,
): Promise<Playlist[]> {
  if (isTauri) {
    return invoke<Playlist[]>('get_category_playlists', {
      source,
      category,
      limit,
    })
  }
  // 浏览器降级
  if (source === 'netease' && category === 'daily') {
    return [
      {
        id: 'netease:daily',
        name: '每日推荐',
        description: '根据你的口味每日更新的推荐歌曲',
        cover_url: 'https://picsum.photos/seed/netease-daily/400/400',
        track_count: 30,
        play_count: 0,
        source,
      },
    ]
  }
  // QQ 排行榜 mock：显示官方榜单名
  if (source === 'qq_music' && category === 'hot') {
    const mockCharts = ['巅峰榜·热歌', '巅峰榜·新歌', '巅峰榜·流行指数', '飙升榜', '国乐榜', '说唱榜']
    return mockCharts.map((name, i) => ({
      id: `qq:toplist:${i + 1}`,
      name,
      description: 'QQ 音乐官方排行榜',
      cover_url: `https://picsum.photos/seed/qq-chart-${i}/400/400`,
      track_count: 0,
      play_count: 1000000 * (i + 1),
      source,
    }))
  }
  return getRecommendedPlaylists(source, limit)
}
