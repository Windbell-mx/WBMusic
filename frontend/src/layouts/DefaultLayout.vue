<template>
  <!-- 标题栏高度 36px，布局从其下方开始。
       非播放路由时底部有 84px 高的播放器（fixed 悬浮），把滚动容器高度同步
       减掉 84px，滚动条轨道就在播放器上方结束，不再与其重合。 -->
  <n-layout
    has-sider
    :style="{
      height: isPlayerRoute ? 'calc(100vh - 36px)' : 'calc(100vh - 120px)',
      marginTop: '36px',
    }"
  >
    <!-- 侧边栏导航 -->
    <n-layout-sider
      bordered
      v-model:collapsed="appStore.sidebarCollapsed"
      :collapsed-width="64"
      :width="220"
      collapse-mode="width"
      show-trigger="bar"
      class="sidebar-glass"
    >
      <n-layout-header class="sidebar-header" @click="handleLogoClick">
        <transition name="fade" mode="out-in">
          <div v-if="appStore.sidebarCollapsed" key="tab" class="logo-tab" title="WBMusic">
            <span>W</span>
          </div>
          <n-h1 v-else key="text" class="logo-text">WBMusic</n-h1>
        </transition>
      </n-layout-header>

      <n-layout-content style="flex: 1; padding: 12px 0">
        <n-menu
          :value="currentRoute"
          :options="menuOptions"
          @update:value="handleMenuClick"
          style="background: transparent"
        />
      </n-layout-content>

      <!-- 第三方接入 -->
      <div v-if="!appStore.sidebarCollapsed" class="third-party">
        <div class="tp-title">第三方接入</div>
        <div class="tp-card qq" @click="openLogin('qq_music', 'QQ 音乐')">
          <div class="tp-icon">
            <n-icon :component="MusicalNote" size="18" />
          </div>
          <div class="tp-info">
            <span class="tp-name">QQ音乐</span>
            <span class="tp-badge" :class="{ on: qqLoggedIn }">{{ qqLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
        </div>
        <div class="tp-card netease" @click="openLogin('netease', '网易云音乐')">
          <div class="tp-icon">
            <n-icon :component="MusicalNotes" size="18" />
          </div>
          <div class="tp-info">
            <span class="tp-name">网易云音乐</span>
            <span class="tp-badge" :class="{ on: neteaseLoggedIn }">{{ neteaseLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
        </div>
      </div>
    </n-layout-sider>

    <!-- 登录弹窗 -->
    <LoginModal
      v-model:show="showLoginModal"
      :provider="loginTarget"
      @changed="refreshProviderStatus"
    />

    <!-- 主内容区：滚动容器高度已减掉播放器高度，底部无需再预留 80px -->
    <n-layout class="main-content-layout" style="position: relative">
      <n-layout-content
        style="padding: 16px 24px 24px"
        content-style="background: var(--n-color);"
      >
        <!-- 内容区：最大化窗口下充分利用宽度（首页内部自适应两栏） -->
        <div class="page-container">
          <router-view :key="appStore.refreshKey" />
        </div>
      </n-layout-content>

      <!-- 底部播放器（播放页为全屏覆盖，故隐藏） -->
      <PlayerBar v-if="!isPlayerRoute" />
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { computed, h, nextTick, onMounted, ref, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  NLayout,
  NLayoutContent,
  NLayoutSider,
  NMenu,
  NH1,
  NIcon,
} from 'naive-ui'
import { Home, List, Search, Settings, MusicalNote, MusicalNotes } from '@vicons/ionicons5'
import PlayerBar from '@/components/Player/PlayerBar.vue'
import LoginModal from '@/components/LoginModal.vue'
import { useAppStore } from '@/stores/app'
import { getLoginStatus, refreshAll, type MusicSource } from '@/api'

const router = useRouter()
const route = useRoute()
const appStore = useAppStore()

// 播放页为全屏覆盖层（Teleport 到 body），此时隐藏底部播放器
const isPlayerRoute = computed(() => route.name === 'Player')

// 第三方接入登录状态
const qqLoggedIn = ref(false)
const neteaseLoggedIn = ref(false)

async function refreshProviderStatus() {
  try {
    const statuses = await getLoginStatus()
    qqLoggedIn.value = statuses.qq_music?.logged_in ?? false
    neteaseLoggedIn.value = statuses.netease?.logged_in ?? false
  } catch {
    // 浏览器降级环境忽略
  }
}

onMounted(async () => {
  // 启动时同步校验登录态：后端会验证 qm_keyst 等会话凭据是否仍有效，
  // 失效的自动登出（清凭据 + session），避免"显示已登录但实际已过期"。
  // 校验完再用最新状态渲染侧边栏。
  try {
    const statuses = await refreshAll()
    qqLoggedIn.value = statuses.qq_music?.logged_in ?? false
    neteaseLoggedIn.value = statuses.netease?.logged_in ?? false
  } catch {
    await refreshProviderStatus()
  }
})

// 全局刷新（F5/刷新按钮）后重新同步侧边栏登录状态
watch(
  () => appStore.refreshKey,
  () => refreshProviderStatus(),
)

// 路由切换后，将主内容滚动容器重置到顶部。
// 原因：详情页/列表页共用同一个外层滚动容器，若不重置，
// 从详情页返回列表页时列表仍停留在原滚动位置（"退到外边也在下面"）。
watch(
  () => route.fullPath,
  async () => {
    await nextTick()
    const pc = document.querySelector('.page-container')
    if (!pc) return
    let el: HTMLElement | null = pc.parentElement
    while (el) {
      const cs = getComputedStyle(el)
      if (
        (cs.overflowY === 'auto' || cs.overflowY === 'scroll') &&
        el.scrollHeight > el.clientHeight + 5
      ) {
        el.scrollTop = 0
        break
      }
      el = el.parentElement
    }
  },
)

// 登录弹窗
const showLoginModal = ref(false)
const loginTarget = ref<{ key: MusicSource; label: string } | null>(null)

function openLogin(key: MusicSource, label: string) {
  loginTarget.value = { key, label }
  showLoginModal.value = true
}

const currentRoute = computed(() => route.name as string)

const menuOptions = [
  {
    key: 'Home',
    label: '首页',
    icon: () => h(NIcon, null, { default: () => h(Home) }),
  },
  {
    key: 'Search',
    label: '搜索',
    icon: () => h(NIcon, null, { default: () => h(Search) }),
  },
  {
    key: 'Playlist',
    label: '我的',
    icon: () => h(NIcon, null, { default: () => h(List) }),
  },
  {
    key: 'Settings',
    label: '设置',
    icon: () => h(NIcon, null, { default: () => h(Settings) }),
  },
]

function handleMenuClick(key: string) {
  router.push({ name: key })
}

function handleLogoClick() {
  router.push({ name: 'Home' })
}
</script>

<style scoped>
/* ---------- 侧边栏半透明背景（去掉毛玻璃，防滚动/切换卡顿） ---------- */
.sidebar-glass {
  /* 大面 backdrop-filter 在 WebView2 滚动/路由切换时每帧重采样背景，
     交互极卡。改用半透明纯色，视觉接近毛玻璃且零重采样开销 */
  background: rgba(255, 255, 255, 0.92) !important;
  box-shadow: inset -1px 0 0 rgba(0, 0, 0, 0.08);

  /* has-sider 模式下 sider 静态定位在滚动容器内，会跟着主内容一起滚动。
     sticky + top:0 让它相对外层滚动容器固定，侧边栏始终保持不动 */
  position: sticky !important;
  top: 0;
  align-self: flex-start;
}

html.dark .sidebar-glass {
  /* 暗色：深色半透明 + 淡紫右描边 */
  background: rgba(28, 28, 34, 0.92) !important;
  box-shadow: inset -1px 0 0 color-mix(in srgb, var(--accent-light) 35%, transparent);
}

/* ---------- 主内容区 ---------- */
.page-container {
  max-width: 1800px;
  margin: 0 auto;
  width: 100%;
}

/* 关键：内层主内容布局及其 scroll container 默认 overflow: hidden/auto，
   会成为子页面 sticky 元素的“最近滚动祖先”但并不真正滚动，
   导致详情页返回按钮无法冻结在顶部。改为 overflow: visible，
   让 sticky 直接相对外层真实滚动容器生效。 */
.main-content-layout,
.main-content-layout :deep(.n-layout-scroll-container),
.main-content-layout :deep(.n-layout-content) {
  overflow: visible;
}

/* ---------- 侧边栏头部 ---------- */
.sidebar-header {
  padding: 20px 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-bottom: 1px solid var(--n-border-color);
  cursor: pointer;
  user-select: none;
}

.logo-text {
  margin: 0;
  white-space: nowrap;
  font-weight: 700;
  font-size: 20px;
  color: var(--accent);
}

/* 收起时的页签徽章 */
.logo-tab {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  border-radius: 12px 12px 12px 4px;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  font-weight: 800;
  letter-spacing: 0.5px;
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 45%, transparent);
  transition: transform 0.2s, box-shadow 0.2s;
}

.logo-tab:hover {
  transform: scale(1.06);
  box-shadow: 0 6px 18px color-mix(in srgb, var(--accent) 55%, transparent);
}

/* 文字/徽章切换过渡 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* ---------- 第三方接入 ---------- */
.third-party {
  padding: 0 12px 16px;
}

.tp-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--n-text-color-3);
  letter-spacing: 0.5px;
  padding: 10px 6px 8px;
  user-select: none;
}

.tp-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  cursor: pointer;
  border: 1px solid var(--n-border-color);
  background: var(--n-color);
  transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
  margin-bottom: 8px;
}

.tp-card:last-child {
  margin-bottom: 0;
}

.tp-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.12);
}

.tp-card:active {
  transform: translateY(0);
}

.tp-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

/* QQ音乐：品牌绿 */
.tp-card.qq .tp-icon {
  background: #31c27c;
}

/* 网易云：品牌红 */
.tp-card.netease .tp-icon {
  background: #d43c33;
}

.tp-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 3px;
  min-width: 0;
}

.tp-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--n-text-color);
  white-space: nowrap;
}

.tp-badge {
  font-size: 10px;
  line-height: 1;
  padding: 2px 6px;
  border-radius: 999px;
  color: var(--n-text-color-3);
  background: var(--n-divider-color);
}

.tp-badge.on {
  color: #fff;
  background: #31c27c;
}
</style>
