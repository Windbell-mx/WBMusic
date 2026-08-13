import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { createDiscreteApi } from 'naive-ui'
import { usePlayerStore } from './player'

/**
 * 全局快捷键 store
 * - 支持 播放/暂停、上一首、下一首、音量增大、音量减小 五个动作
 * - 每个动作可自定义绑定键位（支持组合键与媒体键），localStorage 持久化
 * - 应用窗口聚焦时生效；输入框/文本域聚焦时自动忽略，避免干扰输入
 * - 录制模式下优先捕获按键，避免触发已绑定的动作
 */

/** 可绑定的动作 */
export type ShortcutAction = 'playPause' | 'prev' | 'next' | 'volumeUp' | 'volumeDown'

/** 单个键位绑定 */
export interface ShortcutBinding {
  /** 是否启用（false 表示未设置） */
  enabled: boolean
  /** 主键（规范化小写），空格为 ' '，方向键为 arrowleft 等，媒体键为 mediaplaypause 等 */
  key: string
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
}

/** 全部动作的绑定表 */
export type ShortcutMap = Record<ShortcutAction, ShortcutBinding>

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  'playPause',
  'prev',
  'next',
  'volumeUp',
  'volumeDown',
]

export const ACTION_LABELS: Record<ShortcutAction, string> = {
  playPause: '播放 / 暂停',
  prev: '上一首',
  next: '下一首',
  volumeUp: '音量增大',
  volumeDown: '音量减小',
}

/** 音量单次调节步长 */
const VOLUME_STEP = 5

const STORAGE_KEY = 'wbmusic.shortcuts'
const ENABLED_KEY = 'wbmusic.shortcutsEnabled'

const DEFAULT_BINDINGS: ShortcutMap = {
  playPause: { enabled: true, key: ' ', ctrl: false, alt: false, shift: false, meta: false },
  prev: { enabled: true, key: 'arrowleft', ctrl: true, alt: true, shift: false, meta: false },
  next: { enabled: true, key: 'arrowright', ctrl: true, alt: true, shift: false, meta: false },
  volumeUp: { enabled: true, key: 'arrowup', ctrl: true, alt: true, shift: false, meta: false },
  volumeDown: { enabled: true, key: 'arrowdown', ctrl: true, alt: true, shift: false, meta: false },
}

/** 长按允许重复触发（连续调节音量）的动作 */
const REPEAT_SAFE: ShortcutAction[] = ['volumeUp', 'volumeDown']

/** 键位显示名 */
const KEY_LABELS: Record<string, string> = {
  ' ': '空格',
  arrowleft: '←',
  arrowright: '→',
  arrowup: '↑',
  arrowdown: '↓',
  escape: 'Esc',
  enter: 'Enter',
  tab: 'Tab',
  backspace: 'Backspace',
  delete: 'Delete',
  home: 'Home',
  end: 'End',
  pageup: 'PageUp',
  pagedown: 'PageDown',
  mediaplaypause: '媒体播放/暂停键',
  mediatracknext: '媒体下一曲键',
  mediatrackprevious: '媒体上一曲键',
  mediastop: '媒体停止键',
  volumeup: '音量增大键',
  volumedown: '音量减小键',
  audiovolumeup: '音量增大键',
  audiovolumedown: '音量减小键',
  audioplaypause: '媒体播放/暂停键',
  audiotracknext: '媒体下一曲键',
  audiotrackprevious: '媒体上一曲键',
}

const { message } = createDiscreteApi(['message'])

/** 深拷贝绑定表（避免修改共享的默认常量） */
function cloneBindings(m: ShortcutMap): ShortcutMap {
  const result = {} as ShortcutMap
  for (const action of SHORTCUT_ACTIONS) {
    result[action] = { ...m[action] }
  }
  return result
}

/** 将按键事件的主键规范化为小写字符串（空格为 ' '） */
function normalizeKey(e: KeyboardEvent): string | null {
  if (e.code === 'Space') return ' '
  const key = e.key.toLowerCase()
  // 纯修饰键（如单独按 Ctrl）不是有效主键，等待下一个按键
  if (['control', 'alt', 'shift', 'meta', 'dead', 'unidentified'].includes(key)) return null
  return key
}

/** 判断绑定是否与按键事件匹配 */
function bindingMatches(b: ShortcutBinding, e: KeyboardEvent): boolean {
  if (!b.enabled) return false
  const key = normalizeKey(e)
  if (!key || key !== b.key) return false
  return b.ctrl === e.ctrlKey && b.alt === e.altKey && b.shift === e.shiftKey && b.meta === e.metaKey
}

/** 两个绑定是否等价（用于冲突检测） */
function bindingsEqual(a: ShortcutBinding, b: ShortcutBinding): boolean {
  return (
    a.key === b.key &&
    a.ctrl === b.ctrl &&
    a.alt === b.alt &&
    a.shift === b.shift &&
    a.meta === b.meta
  )
}

/** 将键位格式化为人类可读文本，如 "Ctrl + Alt + →" */
export function formatKey(b: ShortcutBinding): string {
  const parts: string[] = []
  if (b.ctrl) parts.push('Ctrl')
  if (b.alt) parts.push('Alt')
  if (b.shift) parts.push('Shift')
  if (b.meta) parts.push('Meta')
  const key = b.key
  const label = /^f\d+$/.test(key) ? key.toUpperCase() : KEY_LABELS[key] ?? key.toUpperCase()
  parts.push(label)
  return parts.join(' + ')
}

export const useShortcutStore = defineStore('shortcuts', () => {
  /** 当前正在录制键位的动作（null 表示未录制） */
  const recording = ref<ShortcutAction | null>(null)

  /** 从 localStorage 恢复绑定，无记录时使用默认值 */
  function loadBindings(): ShortcutMap {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return cloneBindings(DEFAULT_BINDINGS)
      const parsed = JSON.parse(raw) as Partial<ShortcutMap>
      const result = cloneBindings(DEFAULT_BINDINGS)
      for (const action of SHORTCUT_ACTIONS) {
        const p = parsed[action]
        if (p && typeof p === 'object' && typeof p.key === 'string') {
          result[action] = {
            enabled: !!p.enabled,
            key: p.key,
            ctrl: !!p.ctrl,
            alt: !!p.alt,
            shift: !!p.shift,
            meta: !!p.meta,
          }
        }
      }
      return result
    } catch {
      return cloneBindings(DEFAULT_BINDINGS)
    }
  }

  const bindings = ref<ShortcutMap>(loadBindings())

  /** 快捷键总开关（关闭后所有快捷键不响应，仍保留绑定配置） */
  const enabled = ref(localStorage.getItem(ENABLED_KEY) !== '0')

  // 绑定持久化：任何改动自动落盘
  watch(
    bindings,
    (value) => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
      } catch {
        /* 存储失败忽略 */
      }
    },
    { deep: true },
  )

  // 总开关持久化
  watch(enabled, (value) => {
    try {
      localStorage.setItem(ENABLED_KEY, value ? '1' : '0')
    } catch {
      /* 存储失败忽略 */
    }
  })

  function runAction(action: ShortcutAction) {
    const player = usePlayerStore()
    switch (action) {
      case 'playPause':
        player.togglePlay()
        break
      case 'prev':
        player.prevTrack()
        break
      case 'next':
        player.nextTrack()
        break
      case 'volumeUp':
        player.setVolume(Math.min(100, player.volume + VOLUME_STEP))
        break
      case 'volumeDown':
        player.setVolume(Math.max(0, player.volume - VOLUME_STEP))
        break
    }
  }

  /** 设置绑定并处理冲突（占用同一键位的其他动作被自动禁用） */
  function setBinding(action: ShortcutAction, binding: ShortcutBinding) {
    const conflicted: ShortcutAction[] = []
    for (const other of SHORTCUT_ACTIONS) {
      if (other === action) continue
      const ob = bindings.value[other]
      if (ob.enabled && bindingsEqual(ob, binding)) conflicted.push(other)
    }
    for (const other of conflicted) {
      bindings.value[other] = { ...bindings.value[other], enabled: false }
    }
    bindings.value[action] = { ...binding }
    if (conflicted.length > 0) {
      message.warning(
        `键位与「${conflicted.map((a) => ACTION_LABELS[a]).join('、')}」冲突，已清除该动作的绑定`,
      )
    }
  }

  /** 录制模式下处理按键：Esc 取消；纯修饰键等待；其余作为新键位绑定 */
  function handleRecordingKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      recording.value = null
      return
    }
    if (e.repeat) return
    const key = normalizeKey(e)
    if (!key) return
    const action = recording.value
    if (!action) return
    setBinding(action, {
      enabled: true,
      key,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      shift: e.shiftKey,
      meta: e.metaKey,
    })
    recording.value = null
  }

  /** 全局按键监听：捕获阶段优先处理，保证录制与快捷键不被组件拦截 */
  function onKeydown(e: KeyboardEvent) {
    // 总开关关闭：快捷键全部失效（录制也不响应）
    if (!enabled.value) return
    // 输入框/文本域/可编辑区聚焦时不拦截，避免影响输入
    const target = e.target as HTMLElement | null
    if (
      target &&
      (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)
    ) {
      return
    }
    // 录制状态：优先捕获按键
    if (recording.value) {
      e.preventDefault()
      e.stopPropagation()
      handleRecordingKey(e)
      return
    }
    for (const action of SHORTCUT_ACTIONS) {
      const b = bindings.value[action]
      if (bindingMatches(b, e)) {
        // 播放/暂停、切歌等动作忽略长按重复，音量调节允许连续触发
        if (e.repeat && !REPEAT_SAFE.includes(action)) return
        e.preventDefault()
        runAction(action)
        return
      }
    }
  }

  // 全局监听（捕获阶段），store 单例创建即生效
  window.addEventListener('keydown', onKeydown, true)

  function isRecording(action: ShortcutAction): boolean {
    return recording.value === action
  }

  function startRecord(action: ShortcutAction) {
    recording.value = action
  }

  function cancelRecord() {
    recording.value = null
  }

  function isEnabled(action: ShortcutAction): boolean {
    return bindings.value[action].enabled
  }

  /** 清除指定动作的绑定（禁用） */
  function clear(action: ShortcutAction) {
    bindings.value[action] = { ...bindings.value[action], enabled: false }
  }

  /** 恢复全部默认键位（同时恢复总开关） */
  function resetAll() {
    bindings.value = cloneBindings(DEFAULT_BINDINGS)
    enabled.value = true
  }

  /** 获取动作的格式化键位文本 */
  function format(action: ShortcutAction): string {
    return formatKey(bindings.value[action])
  }

  return {
    enabled,
    recording,
    bindings,
    isRecording,
    startRecord,
    cancelRecord,
    isEnabled,
    clear,
    resetAll,
    format,
  }
})
