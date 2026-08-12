import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { getCacheDir, isTauri } from '@/api'

/** 主题模式：system 跟随系统 / light 固定浅色 / dark 固定深色 */
export type ThemeMode = 'system' | 'light' | 'dark'

const THEME_MODE_KEY = 'wbmusic.themeMode'

function applyDarkClass(isDark: boolean) {
  document.documentElement.classList.toggle('dark', isDark)
}

export const useAppStore = defineStore('app', () => {
  const isDark = ref(false)
  // ---- 设置项（localStorage 持久化，定义时恢复）----
  const themeColor = ref(localStorage.getItem('wbmusic.themeColor') || '#667eea')
  const sidebarCollapsed = ref(localStorage.getItem('wbmusic.sidebarCollapsed') === '1')
  const autoPlay = ref(localStorage.getItem('wbmusic.autoPlay') !== '0')
  const showLyrics = ref(localStorage.getItem('wbmusic.showLyrics') !== '0')

  /** 主题模式：跟随系统 / 浅色 / 深色（localStorage 持久化） */
  const themeMode = ref<ThemeMode>('system')

  /** 系统深浅色 MediaQuery 监听器（仅跟随系统模式时激活） */
  let systemMedia: MediaQueryList | null = null

  /** 缓存路径：Tauri 环境下为完整绝对路径；浏览器降级为相对路径 */
  const cachePath = ref('WBMusic/cache')

  /** 全局刷新计数器：每次全局刷新 +1，router-view :key 绑定它即可强制重挂载当前页面 */
  const refreshKey = ref(0)

  /** 触发全局刷新（递增计数器，页面组件重新挂载并拉取最新数据） */
  function triggerRefresh() {
    refreshKey.value += 1
  }

  // 初始化缓存路径：
  // 1. 用户手动设置过 → 直接使用
  // 2. Tauri 环境 → 从后端获取缓存目录（默认在安装位置下，如 D:\Program Files\WBMusic\cache）
  // 3. 浏览器降级 → WBMusic/cache
  async function initCachePath() {
    const saved = localStorage.getItem('wbmusic.cachePath')
    if (saved) {
      cachePath.value = saved
      return
    }
    if (isTauri) {
      try {
        cachePath.value = await getCacheDir()
      } catch {
        /* 后端不可用时保持默认 */
      }
    }
  }
  initCachePath()

  function toggleTheme() {
    setThemeMode(isDark.value ? 'light' : 'dark')
  }

  function setDark(value: boolean) {
    setThemeMode(value ? 'dark' : 'light')
  }

  // ---- 主题模式（跟随系统 / 浅色 / 深色）----

  /** 从 localStorage 恢复主题模式并应用，跟随系统时监听系统变化 */
  function initThemeMode() {
    const saved = localStorage.getItem(THEME_MODE_KEY)
    themeMode.value = saved === 'light' || saved === 'dark' ? saved : 'system'
    if (themeMode.value === 'system') {
      followSystemTheme()
    } else {
      applyDark(themeMode.value === 'dark')
    }
  }

  /** 应用最终主题（Naive UI + 全局 .dark class + 原生控件 color-scheme） */
  function applyDark(value: boolean) {
    isDark.value = value
    applyDarkClass(value)
    document.documentElement.style.colorScheme = value ? 'dark' : 'light'
  }

  /** 跟随系统深浅色，并监听系统设置变化实时切换 */
  function followSystemTheme() {
    if (!window.matchMedia) {
      applyDark(false)
      return
    }
    systemMedia?.removeEventListener?.('change', onSystemThemeChange)
    systemMedia = window.matchMedia('(prefers-color-scheme: dark)')
    systemMedia.addEventListener?.('change', onSystemThemeChange)
    applyDark(systemMedia.matches)
  }

  function onSystemThemeChange(event: MediaQueryListEvent) {
    applyDark(event.matches)
  }

  /** 设置主题模式：'system' 跟随系统 / 'light' 浅色 / 'dark' 深色 */
  function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode
    localStorage.setItem(THEME_MODE_KEY, mode)
    if (mode === 'system') {
      followSystemTheme()
    } else {
      systemMedia?.removeEventListener?.('change', onSystemThemeChange)
      applyDark(mode === 'dark')
    }
  }

  function setThemeColor(color: string) {
    themeColor.value = color
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setCachePath(path: string) {
    cachePath.value = path
    localStorage.setItem('wbmusic.cachePath', path)
  }

  // ---- 设置项持久化：任何改动（含组件直接赋值）统一落盘 ----
  watch([themeColor, sidebarCollapsed, autoPlay, showLyrics], ([c, s, a, l]) => {
    localStorage.setItem('wbmusic.themeColor', c)
    localStorage.setItem('wbmusic.sidebarCollapsed', s ? '1' : '0')
    localStorage.setItem('wbmusic.autoPlay', a ? '1' : '0')
    localStorage.setItem('wbmusic.showLyrics', l ? '1' : '0')
  })

  // 初始化主题模式（缓存路径已在定义处初始化）
  initThemeMode()

  return {
    isDark,
    themeMode,
    themeColor,
    sidebarCollapsed,
    autoPlay,
    showLyrics,
    cachePath,
    refreshKey,
    triggerRefresh,
    toggleTheme,
    setDark,
    setThemeMode,
    setThemeColor,
    toggleSidebar,
    setCachePath,
    initCachePath,
  }
})
