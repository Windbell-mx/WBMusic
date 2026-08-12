import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { Track } from '@/api'
import { isTauri, likeTrack } from '@/api'
import { createDiscreteApi } from 'naive-ui'
import { useAppStore } from './app'

/** localStorage 播放记录键名 */
const STORAGE_KEY = 'wbmusic:player-state'

/** 浏览器调试环境播放失败时的演示音频（仅非 Tauri 环境使用） */
const DEMO_AUDIO_URL = 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3'

/** 全局消息提示（discrete API，可在非组件环境调用） */
const { message } = createDiscreteApi(['message'])

interface SavedPlayerState {
  version: number
  playlist: Track[]
  currentIndex: number
  /** 上次播放进度（秒） */
  currentTime: number
  volume: number
  playMode: 'list' | 'one' | 'shuffle'
  isLiked: boolean
}

/**
 * 全局播放状态 store
 * - 管理唯一的 Audio 实例与播放列表
 * - 底部 PlayerBar 与全屏 PlayerView 共享同一状态
 * - 播放记录持久化到 localStorage，下次启动自动恢复续播
 */
export const usePlayerStore = defineStore('player', () => {
  // ---- 播放列表与当前曲目 ----
  const playlist = ref<Track[]>([])
  const currentIndex = ref(-1)

  const currentTrack = computed<Track | null>(() =>
    currentIndex.value >= 0 ? playlist.value[currentIndex.value] ?? null : null,
  )

  /** 下一首歌曲（按当前播放模式推算；shuffle 无法确定时按顺序下一首作为参考） */
  const nextTrackInfo = computed<Track | null>(() => {
    if (playlist.value.length === 0 || currentIndex.value < 0) return null
    if (playMode.value === 'one') return currentTrack.value
    const next = (currentIndex.value + 1) % playlist.value.length
    return playlist.value[next] ?? null
  })

  // ---- 播放状态 ----
  const isPlaying = ref(false)
  const progress = ref(0) // 0-100
  const currentSec = ref(0) // 音频真实当前秒数（歌词同步用）
  const volume = ref(30) // 0-100
  const isLiked = ref(false)

  // ---- 播放模式 ----
  type PlayMode = 'list' | 'one' | 'shuffle'
  const playMode = ref<PlayMode>('list')

  // ---- 全局设置（自动播放开关等）----
  const appStore = useAppStore()

  /** 连续播放失败计数（防止自动跳歌死循环） */
  let consecutiveFailures = 0

  // ---- 全局唯一音频实例 ----
  const audio = new Audio()
  let audioUrl = ''
  let totalSeconds = 0
  /** 音频真实时长（秒），响应式，用于 totalTime 显示 */
  const audioDuration = ref(0)

  // ---- 播放记录（持久化）----
  let resumeTime = 0 // 恢复进度（秒），仅启动恢复时消费一次
  let lastSavedTime = 0 // 最近一次保存的播放进度（秒）
  let saveTimer: ReturnType<typeof setTimeout> | null = null

  audio.volume = volume.value / 100

  // ---- 播放记录：保存 / 调度 / 恢复 ----
  function saveState() {
    try {
      const state: SavedPlayerState = {
        version: 1,
        playlist: playlist.value,
        currentIndex: currentIndex.value,
        currentTime: lastSavedTime,
        volume: volume.value,
        playMode: playMode.value,
        isLiked: isLiked.value,
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      /* 存储失败忽略（如隐私模式） */
    }
  }

  function scheduleSave(delay = 300) {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(saveState, delay)
  }

  function restoreState() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return
      const s = JSON.parse(raw) as Partial<SavedPlayerState>
      if (Array.isArray(s.playlist)) {
        playlist.value = s.playlist.filter(
          (t): t is Track =>
            !!t &&
            typeof t.id !== 'undefined' &&
            typeof t.title === 'string' &&
            !String(t.id).includes('-mock-'), // 过滤历史遗留的演示歌曲
        )
      }
      if (
        typeof s.currentIndex === 'number' &&
        s.currentIndex >= 0 &&
        s.currentIndex < playlist.value.length
      ) {
        currentIndex.value = s.currentIndex
      }
      if (typeof s.volume === 'number' && s.volume >= 0 && s.volume <= 100) {
        volume.value = s.volume
        audio.volume = s.volume / 100
      }
      if (
        s.playMode === 'list' ||
        s.playMode === 'one' ||
        s.playMode === 'shuffle'
      ) {
        playMode.value = s.playMode
      }
      if (typeof s.isLiked === 'boolean') isLiked.value = s.isLiked
      if (typeof s.currentTime === 'number' && s.currentTime > 0) {
        resumeTime = s.currentTime
        lastSavedTime = s.currentTime
      }
    } catch {
      /* 数据损坏时忽略 */
    }
  }

  // ---- 音频事件 ----
  audio.addEventListener('durationchange', () => {
    if (audio.duration && !Number.isNaN(audio.duration)) {
      audioDuration.value = audio.duration
      totalSeconds = audio.duration
    }
  })
  audio.addEventListener('timeupdate', () => {
    // 无条件记录当前进度（duration 未知时也保存，避免流媒体加载期丢进度）
    lastSavedTime = audio.currentTime
    currentSec.value = audio.currentTime
    if (audio.duration && !Number.isNaN(audio.duration)) {
      progress.value = (audio.currentTime / audio.duration) * 100
      totalSeconds = audio.duration
      audioDuration.value = audio.duration
    }
    // 播放进度 2 秒防抖保存
    scheduleSave(2000)
  })
  audio.addEventListener('play', () => {
    isPlaying.value = true
  })
  audio.addEventListener('pause', () => {
    isPlaying.value = false
    // 暂停时立即保存进度（不依赖防抖，避免关闭应用时丢进度）
    saveState()
  })
  audio.addEventListener('ended', () => {
    if (playMode.value === 'one') {
      audio.currentTime = 0
      audio.play()
    } else {
      nextTrack()
    }
  })

  // ---- 播放控制 ----
  /**
   * 用新歌单替换当前播放列表并播放（歌单播放不叠加历史队列）
   * @param tracks 歌单歌曲（API Track）
   * @param startIndex 从歌单第几首开始（默认 0）
   */
  function playPlaylist(tracks: Track[], startIndex = 0) {
    audio.pause()
    playlist.value = tracks.filter(
      (t) => !!t && typeof t.id !== 'undefined' && typeof t.title === 'string',
    )
    currentIndex.value =
      playlist.value.length === 0
        ? -1
        : Math.min(Math.max(startIndex, 0), playlist.value.length - 1)
    if (currentIndex.value >= 0) loadCurrent()
  }

  function playTrack(track: Track, _index?: number) {
    // 始终按 id + source 在全局播放列表中查找实际位置，
    // 不能信任外部传入的歌单内下标（全局 playlist 会累积多个歌单的歌曲，下标会错位）
    const idx = playlist.value.findIndex(
      (t) => t.id === track.id && t.source === track.source,
    )
    if (idx >= 0) {
      currentIndex.value = idx
    } else {
      playlist.value.push(track)
      currentIndex.value = playlist.value.length - 1
    }
    loadCurrent()
  }

  /**
   * 下一首播放：把歌曲插入到当前播放位置之后并立即播放
   * （已在队列中则直接跳到该歌曲）
   */
  function playNext(track: Track) {
    const idx = playlist.value.findIndex(
      (t) => t.id === track.id && t.source === track.source,
    )
    if (idx >= 0) {
      currentIndex.value = idx
    } else {
      const insertAt =
        currentIndex.value >= 0
          ? currentIndex.value + 1
          : playlist.value.length
      playlist.value.splice(insertAt, 0, track)
      currentIndex.value = insertAt
    }
    loadCurrent()
  }

  /** 真机环境播放失败处理：提示失败原因并自动跳过（带死循环保护） */
  function handleLoadFailure(track: Track, e: unknown) {
    consecutiveFailures++
    const reason =
      e instanceof Error && e.message && e.message !== '未获取到有效播放地址'
        ? e.message
        : '可能为 VIP 专享或无版权'
    // 播放列表只剩一首，或整圈全部失败：停止播放，避免无限跳歌
    if (playlist.value.length <= 1 || consecutiveFailures >= playlist.value.length) {
      consecutiveFailures = 0
      audio.pause()
      audio.removeAttribute('src')
      message.warning(`「${track.title}」无法播放：${reason}`)
      return
    }
    message.error(`「${track.title}」播放失败已自动跳过：${reason}`)
    nextTrack()
  }

  async function loadCurrent(autoPlay = true) {
    const track = currentTrack.value
    if (!track) return
    // 切歌/加载时重置进度显示，等 durationchange 后更新
    progress.value = 0
    totalSeconds = 0
    audioDuration.value = 0
    try {
      const { getTrackUrl } = await import('@/api')
      const url = await getTrackUrl(track.source, String(track.id))
      if (!url || !url.startsWith('http')) {
        throw new Error('未获取到有效播放地址')
      }
      if (audioUrl) URL.revokeObjectURL(audioUrl)
      audioUrl = url
      audio.src = url
      audio.load()
      applyResumeSeek()
      if (autoPlay) {
        audio.play().catch(() => {
          /* 自动播放被浏览器拦截时静默 */
        })
      }
      consecutiveFailures = 0 // 播放成功，重置失败计数
    } catch (e) {
      if (isTauri) {
        // 真机环境：明确提示并自动跳过，绝不静默播放测试音频
        handleLoadFailure(track, e)
        return
      }
      // 浏览器调试环境：回退演示音频，保证开发体验
      audio.src = DEMO_AUDIO_URL
      audioUrl = ''
      audio.load()
      applyResumeSeek()
      if (autoPlay) {
        audio.play().catch(() => {
          /* 自动播放被浏览器拦截时静默 */
        })
      }
    }
  }

  /** 应用恢复的播放进度（启动恢复时消费一次） */
  function applyResumeSeek() {
    if (resumeTime <= 0) return
    const target = resumeTime
    resumeTime = 0
    const onMeta = () => {
      audio.removeEventListener('loadedmetadata', onMeta)
      if (audio.duration && !Number.isNaN(audio.duration) && target < audio.duration) {
        audio.currentTime = target
        progress.value = (target / audio.duration) * 100
        totalSeconds = audio.duration
        audioDuration.value = audio.duration
        lastSavedTime = target
      }
    }
    audio.addEventListener('loadedmetadata', onMeta)
  }

  function togglePlay() {
    if (!currentTrack.value) return
    if (audio.paused) {
      audio.play()
    } else {
      audio.pause()
    }
  }

  function seek(percent: number) {
    progress.value = percent
    if (audio.duration && !Number.isNaN(audio.duration)) {
      audio.currentTime = (percent / 100) * audio.duration
      // 拖动进度条（含暂停时拖动）后立即记录新位置并保存
      lastSavedTime = audio.currentTime
      currentSec.value = audio.currentTime
      scheduleSave()
    }
  }

  /** 按秒跳转（歌词点击跳转用） */
  function seekTo(seconds: number) {
    if (!audio.duration || Number.isNaN(audio.duration)) return
    if (seconds < 0) seconds = 0
    if (seconds > audio.duration) seconds = audio.duration
    audio.currentTime = seconds
    progress.value = (seconds / audio.duration) * 100
    lastSavedTime = seconds
    currentSec.value = seconds
    scheduleSave()
  }

  function setVolume(value: number) {
    volume.value = value
    audio.volume = value / 100
  }

  function prevTrack() {
    if (playlist.value.length === 0) return
    currentIndex.value =
      (currentIndex.value - 1 + playlist.value.length) % playlist.value.length
    loadCurrent(appStore.autoPlay)
  }

  function nextTrack() {
    if (playlist.value.length === 0) return
    if (playMode.value === 'shuffle' && playlist.value.length > 1) {
      let next = Math.floor(Math.random() * playlist.value.length)
      if (next === currentIndex.value) next = (next + 1) % playlist.value.length
      currentIndex.value = next
    } else {
      currentIndex.value = (currentIndex.value + 1) % playlist.value.length
    }
    loadCurrent(appStore.autoPlay)
  }

  function cyclePlayMode() {
    const order: PlayMode[] = ['list', 'one', 'shuffle']
    const index = order.indexOf(playMode.value)
    playMode.value = order[(index + 1) % order.length]
  }

  function setPlayMode(mode: PlayMode) {
    playMode.value = mode
  }

  function toggleLike() {
    // 需要当前曲目
    const track = currentTrack.value
    if (!track) return
    const target = !isLiked.value
    // 乐观更新
    isLiked.value = target
    // 浏览器环境直接模拟成功
    if (!isTauri) {
      scheduleSave()
      return
    }
    likeTrack(track.source, track.id, target)
      .then(() => {
        message.success(target ? '已收藏到默认喜欢歌单' : '已取消收藏')
        scheduleSave()
      })
      .catch((err: unknown) => {
        // 失败回滚
        isLiked.value = !target
        message.error(`收藏失败：${err instanceof Error ? err.message : String(err)}`)
      })
  }

  function pauseAll() {
    audio.pause()
  }

  /** 从播放列表移除指定位置的歌曲；若移除的是当前曲目则自动续播下一首 */
  function removeTrack(index: number) {
    if (index < 0 || index >= playlist.value.length) return
    playlist.value.splice(index, 1)
    if (index < currentIndex.value) {
      currentIndex.value--
    } else if (index === currentIndex.value) {
      if (playlist.value.length === 0) {
        // 队列清空：停止播放
        currentIndex.value = -1
        audio.pause()
        if (audioUrl) {
          URL.revokeObjectURL(audioUrl)
          audioUrl = ''
        }
        audio.removeAttribute('src')
      } else {
        currentIndex.value = Math.min(currentIndex.value, playlist.value.length - 1)
        loadCurrent()
      }
    }
  }

  /** 清空播放列表 */
  function clearPlaylist() {
    playlist.value = []
    currentIndex.value = -1
    audio.pause()
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl)
      audioUrl = ''
    }
    audio.removeAttribute('src')
  }

  // ---- 派生 ----
  const currentTime = computed(() => {
    const total = totalSeconds > 0 ? totalSeconds : audioDuration.value
    if (!total) return '0:00'
    const secs = Math.floor((progress.value / 100) * total)
    const minutes = Math.floor(secs / 60)
    const seconds = secs % 60
    return minutes + ':' + seconds.toString().padStart(2, '0')
  })

  const totalTime = computed(() => {
    // 优先用音频真实时长（响应式），其次歌单元数据时长
    const secs = audioDuration.value > 0 ? audioDuration.value : (currentTrack.value?.duration as number | undefined) ?? 0
    if (!secs || Number.isNaN(secs)) return '0:00'
    const s = Math.floor(secs)
    return Math.floor(s / 60) + ':' + (s % 60).toString().padStart(2, '0')
  })

  const modeTitleMap: Record<PlayMode, string> = {
    list: '列表循环',
    one: '单曲循环',
    shuffle: '随机播放',
  }
  const modeTitle = computed(() => modeTitleMap[playMode.value])
  const nextModeTitle = computed(() => {
    const order: PlayMode[] = ['list', 'one', 'shuffle']
    const index = order.indexOf(playMode.value)
    return modeTitleMap[order[(index + 1) % order.length]]
  })

  // ---- 初始化：恢复上次播放记录（仅恢复歌曲与进度，不自动播放）----
  restoreState()
  if (currentIndex.value >= 0) {
    loadCurrent(false)
  }

  // ---- 状态变化自动持久化 ----
  watch(
    [currentIndex, volume, playMode, isLiked],
    () => scheduleSave(),
  )
  watch(playlist, () => scheduleSave(), { deep: true })

  // 应用关闭前兜底保存（Tauri 正常关闭时触发；强杀进程无法拦截）
  const onBeforeUnload = () => saveState()
  window.addEventListener('pagehide', onBeforeUnload)
  window.addEventListener('beforeunload', onBeforeUnload)
  // Tauri 关闭窗口时 WebView2 可能只触发 visibilitychange(hidden) 而不触发 pagehide，
  // 加一层兜底确保关闭前保存最新进度
  const onVisibility = () => {
    if (document.visibilityState === 'hidden') saveState()
  }
  document.addEventListener('visibilitychange', onVisibility)

  return {
    playlist,
    currentIndex,
    currentTrack,
    nextTrackInfo,
    isPlaying,
    progress,
    currentSec,
    audioDuration,
    volume,
    isLiked,
    playMode,
    currentTime,
    totalTime,
    modeTitle,
    nextModeTitle,
    playTrack,
    playNext,
    playPlaylist,
    togglePlay,
    seek,
    seekTo,
    setVolume,
    prevTrack,
    nextTrack,
    cyclePlayMode,
    setPlayMode,
    toggleLike,
    pauseAll,
    removeTrack,
    clearPlaylist,
  }
})
