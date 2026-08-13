/**
 * 数据缓存模块
 *
 * 三级缓存策略：内存缓存（最快）→ 磁盘缓存（Tauri 缓存路径 / 浏览器 localStorage）→ 网络请求。
 * 首页、我的歌单等页面通过该模块读取数据，避免每次切换页面都重新请求，
 * 缓存过期（TTL）后自动回源刷新。
 */

import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '@/api'
import { useAppStore } from '@/stores/app'

/** 缓存条目：数据 + 写入时间戳 */
interface CacheEntry<T> {
  data: T
  savedAt: number
}

/** 内存缓存 Map */
const memoryCache = new Map<string, CacheEntry<unknown>>()

/** localStorage 键前缀（浏览器降级） */
const LS_PREFIX = 'wbmusic:cache:'

/** 各类数据的默认缓存时长（毫秒） */
export const CACHE_TTL = {
  /** 首页发现音乐：15 分钟 */
  home: 15 * 60 * 1000,
  /** 我的歌单：5 分钟 */
  playlist: 5 * 60 * 1000,
} as const

/** 当前缓存目录（来自设置中的「缓存存放路径」） */
function cacheDir(): string {
  return useAppStore().cachePath
}

/** 缓存上限（字节）。0 表示不限制。来自设置中的「缓存上限」（GB） */
function cacheLimitBytes(): number {
  const gb = useAppStore().cacheLimitGB
  return gb > 0 ? gb * 1024 * 1024 * 1024 : 0
}

/**
 * 统计磁盘 / localStorage 中缓存的总大小（字节）
 * 用于设置页展示当前缓存占用。
 */
export async function getCacheSize(): Promise<number> {
  try {
    if (isTauri) {
      return await invoke<number>('cache_dir_size', { dir: cacheDir() })
    }
    let total = 0
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i)
      if (k && k.startsWith(LS_PREFIX)) {
        // localStorage 存的是 UTF-16 字符串，按 2 字节近似估算
        total += (localStorage.getItem(k) || '').length * 2
      }
    }
    return total
  } catch {
    return 0
  }
}

/**
 * 写入磁盘缓存后检查是否超过上限，超过则按「最旧优先」清理，
 * 直到总大小不超过上限（0 = 不限制，直接跳过）。
 */
async function pruneDiskCache(): Promise<void> {
  const limit = cacheLimitBytes()
  if (limit <= 0) return
  try {
    if (isTauri) {
      await invoke('prune_cache_files', {
        dir: cacheDir(),
        maxBytes: limit,
      })
    } else {
      // 浏览器：遍历所有缓存条目，按保存时间从旧到新删除直到不超限
      const items: { key: string; savedAt: number; size: number }[] = []
      let total = 0
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i)
        if (k && k.startsWith(LS_PREFIX)) {
          const raw = localStorage.getItem(k) || ''
          const size = raw.length * 2
          total += size
          let savedAt = 0
          try {
            savedAt = (JSON.parse(raw) as CacheEntry<unknown>).savedAt || 0
          } catch {
            /* 忽略损坏条目 */
          }
          items.push({ key: k, savedAt, size })
        }
      }
      items.sort((a, b) => a.savedAt - b.savedAt)
      for (const item of items) {
        if (total <= limit) break
        localStorage.removeItem(item.key)
        memoryCache.delete(item.key.slice(LS_PREFIX.length))
        total -= item.size
      }
    }
  } catch {
    /* 清理失败不影响主流程 */
  }
}

/**
 * 同步读取内存缓存（命中且未过期返回数据，否则 null）
 * 用于页面挂载时先秒显缓存，避免 loading 闪烁。
 */
export function cachePeek<T>(key: string, ttlMs: number): T | null {
  const entry = memoryCache.get(key)
  if (entry && Date.now() - entry.savedAt < ttlMs) {
    return entry.data as T
  }
  return null
}

/** 异步读取磁盘 / localStorage 缓存（命中且未过期返回数据） */
async function readDisk<T>(key: string, ttlMs: number): Promise<T | null> {
  try {
    if (isTauri) {
      const raw = await invoke<string | null>('read_cache_file', {
        dir: cacheDir(),
        key,
      })
      if (!raw) return null
      const entry = JSON.parse(raw) as CacheEntry<T>
      return Date.now() - entry.savedAt < ttlMs ? entry.data : null
    }
    const raw = localStorage.getItem(LS_PREFIX + key)
    if (!raw) return null
    const entry = JSON.parse(raw) as CacheEntry<T>
    return Date.now() - entry.savedAt < ttlMs ? entry.data : null
  } catch {
    return null
  }
}

/** 写入磁盘 / localStorage 缓存（失败静默，不影响主流程） */
async function writeDisk(key: string, data: unknown): Promise<void> {
  const entry: CacheEntry<unknown> = { data, savedAt: Date.now() }
  try {
    if (isTauri) {
      await invoke('write_cache_file', {
        dir: cacheDir(),
        key,
        content: JSON.stringify(entry),
      })
    } else {
      localStorage.setItem(LS_PREFIX + key, JSON.stringify(entry))
    }
  } catch {
    /* 写缓存失败不影响主流程 */
  }
  // 写入后按上限清理最旧缓存（不阻塞返回）
  pruneDiskCache()
}

/**
 * 获取缓存数据：
 * 1. 内存缓存命中 → 直接返回
 * 2. 磁盘缓存命中 → 返回并回填内存
 * 3. 都未命中 → 调用 fetcher 拉取，写入内存 + 磁盘
 *
 * @param key 缓存键（唯一标识一类数据）
 * @param fetcher 数据源拉取函数（未命中缓存时调用）
 * @param ttlMs 缓存有效期（毫秒）
 * @param force 为 true 时跳过缓存强制拉取并刷新缓存（如手动刷新）
 */
export async function getCached<T>(
  key: string,
  fetcher: () => Promise<T>,
  ttlMs: number,
  force = false,
): Promise<T> {
  // 1. 内存缓存
  if (!force) {
    const mem = cachePeek<T>(key, ttlMs)
    if (mem !== null) return mem
  }

  // 2. 磁盘缓存
  if (!force) {
    const disk = await readDisk<T>(key, ttlMs)
    if (disk !== null) {
      memoryCache.set(key, { data: disk, savedAt: Date.now() })
      return disk
    }
  }

  // 3. 回源拉取并写缓存
  const data = await fetcher()
  memoryCache.set(key, { data, savedAt: Date.now() })
  writeDisk(key, data)
  return data
}

/** 使单个缓存键失效（内存 + 磁盘） */
export async function invalidateCache(key: string): Promise<void> {
  memoryCache.delete(key)
  try {
    if (isTauri) {
      await invoke('remove_cache_file', { dir: cacheDir(), key })
    } else {
      localStorage.removeItem(LS_PREFIX + key)
    }
  } catch {
    /* 忽略 */
  }
}

/** 清空全部缓存（内存 + 磁盘） */
export async function clearAllCache(): Promise<void> {
  memoryCache.clear()
  try {
    if (isTauri) {
      await invoke('clear_cache_files', { dir: cacheDir() })
    } else {
      Object.keys(localStorage)
        .filter((k) => k.startsWith(LS_PREFIX))
        .forEach((k) => localStorage.removeItem(k))
    }
  } catch {
    /* 忽略 */
  }
}
