<template>
  <div class="title-bar" :class="{ 'is-player': isPlayerRoute }">
    <!-- 左侧拖拽区域（双击最大化，与应用图标/标题同行） -->
    <div class="drag-region" data-tauri-drag-region>
      <div class="app-title" data-tauri-drag-region>
        <span class="app-dot" :style="{ background: appStore.themeColor }"></span>
        <span class="app-name" data-tauri-drag-region>{{ windowTitle }}</span>
      </div>
    </div>

    <!-- 右侧窗口控制按钮 -->
    <div class="window-controls">
      <button class="wc-btn wc-refresh" title="刷新 (F5)" @click="handleRefresh" :disabled="refreshing">
        <svg
          class="refresh-icon"
          :class="{ spinning: refreshing }"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <polyline points="21 3 21 9 15 9" />
        </svg>
      </button>
      <!-- 浅色/暗色主题切换 -->
      <button class="wc-btn" :title="appStore.isDark ? '切换到浅色' : '切换到暗色'" @click="appStore.toggleTheme()">
        <svg v-if="appStore.isDark" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="4" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>
      <button class="wc-btn" title="最小化" @click="minimize">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="1" y1="5" x2="9" y2="5" stroke="currentColor" stroke-width="1.2" />
        </svg>
      </button>
      <button class="wc-btn" :title="isMaximized ? '还原' : '最大化'" @click="toggleMaximize">
        <svg v-if="!isMaximized" width="10" height="10" viewBox="0 0 10 10">
          <rect x="1" y="1" width="8" height="8" fill="none" stroke="currentColor" stroke-width="1.2" />
        </svg>
        <svg v-else width="10" height="10" viewBox="0 0 10 10">
          <path
            d="M2 2 h5 v5 h-5 z M3.5 3.5 h4 v4"
            fill="none"
            stroke="currentColor"
            stroke-width="1.2"
          />
        </svg>
      </button>
      <button class="wc-btn wc-close" title="关闭" @click="closeWindow">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.2" />
          <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.2" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { isTauri, refreshAll } from '@/api'

const appStore = useAppStore()
const route = useRoute()

// 播放页（全屏覆盖层）时标题栏与深色播放页背景融合
const isPlayerRoute = computed(() => route.name === 'Player')

// 窗口标题（与 tauri.conf.json 一致）
const windowTitle = 'WBMusic'

// 刷新按钮旋转动画状态
const refreshing = ref(false)

// 全局刷新：后端校验登录态（失效的自动登出）+ 强制重挂载当前页面
async function handleRefresh() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    await refreshAll()
  } catch {
    /* 后端不可用时忽略，仅刷新页面 */
  }
  appStore.triggerRefresh()
  setTimeout(() => {
    refreshing.value = false
  }, 700)
}

// F5 快捷键触发全局刷新（拦截浏览器默认刷新，避免整页重载丢播放状态）
function onKeydown(e: KeyboardEvent) {
  if ((e.key === 'F5') || ((e.ctrlKey || e.metaKey) && (e.key === 'r' || e.key === 'R'))) {
    e.preventDefault()
    handleRefresh()
  }
}

// 最大化状态（用于切换 最大化/还原 图标）
const isMaximized = ref(false)

// Tauri 环境下才加载 window API；浏览器降级为 no-op
let appWindow: import('@tauri-apps/api/window').Window | null = null
let unlistenResize: (() => void) | null = null

async function refreshMaximizeState() {
  if (!appWindow) return
  try {
    isMaximized.value = await appWindow.isMaximized()
  } catch {
    /* 忽略 */
  }
}

async function minimize() {
  try {
    await appWindow?.minimize()
  } catch {
    /* 忽略 */
  }
}

async function toggleMaximize() {
  try {
    await appWindow?.toggleMaximize()
  } catch {
    /* 忽略 */
  }
}

async function closeWindow() {
  try {
    await appWindow?.close()
  } catch {
    /* 忽略 */
  }
}

onMounted(async () => {
  if (!isTauri) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  appWindow = getCurrentWindow()
  await refreshMaximizeState()
  // 窗口尺寸变化（含最大化/还原）后同步图标状态
  unlistenResize = await appWindow.onResized(refreshMaximizeState)
})

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  unlistenResize?.()
  window.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.title-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 36px;
  z-index: 1200;
  display: flex;
  align-items: stretch;
  /* naive-ui 变量（--n-color 等）只注入到组件元素上，标题栏是自定义组件拿不到，
     故按主题硬编码：浅色白底 / 暗色与 naive dark 的 bodyColor(#101014) 一致 */
  background: #fff;
  border-bottom: 1px solid #e5e4e7;
  user-select: none;
  -webkit-user-select: none;
}
html.dark .title-bar {
  background: #101014;
  border-bottom: 1px solid rgba(255, 255, 255, 0.09);
}

/* 播放页：与深色播放页背景融合
   （用 !important 覆盖暗色 html.dark .title-bar 的黑色底，
     否则暗色下标题栏会变成黑色与播放页渐变背景不融合） */
.title-bar.is-player {
  background: transparent !important;
  border-bottom: none !important;
}
.title-bar.is-player .app-name {
  color: rgba(255, 255, 255, 0.9);
}
.title-bar.is-player .wc-btn {
  color: rgba(255, 255, 255, 0.85);
}
.title-bar.is-player .wc-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}
.title-bar.is-player .wc-close:hover {
  background: #e81123;
  color: #fff;
}

/* 拖拽区域（占满剩余空间） */
.drag-region {
  flex: 1;
  display: flex;
  align-items: center;
  padding: 0 12px;
  cursor: default;
}

.app-title {
  display: flex;
  align-items: center;
  gap: 7px;
  pointer-events: none;
}

.app-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  flex-shrink: 0;
}

.app-name {
  font-size: 12.5px;
  font-weight: 600;
  color: #1f1f1f;
  letter-spacing: 0.3px;
  white-space: nowrap;
}
html.dark .title-bar .app-name {
  color: rgba(255, 255, 255, 0.82);
}

/* 窗口控制按钮 */
.window-controls {
  display: flex;
  align-items: center;
  height: 100%;
}

.wc-btn {
  width: 46px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: #1f1f1f;
  cursor: pointer;
  padding: 0;
  transition: background 0.15s;
}
html.dark .title-bar .wc-btn {
  color: rgba(255, 255, 255, 0.82);
}

.wc-btn:hover {
  background: rgba(128, 128, 128, 0.15);
}

/* 刷新按钮 */
.wc-refresh:disabled {
  opacity: 0.8;
  cursor: default;
}

.refresh-icon {
  transition: transform 0.3s ease;
}

.refresh-icon.spinning {
  animation: wbmusic-refresh-spin 0.7s ease;
}

@keyframes wbmusic-refresh-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* 关闭按钮悬停变红 */
.wc-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
