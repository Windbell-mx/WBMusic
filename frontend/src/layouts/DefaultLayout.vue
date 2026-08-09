<template>
  <n-layout has-sider style="height: 100vh">
    <!-- 侧边栏导航 -->
    <n-layout-sider
      bordered
      v-model:collapsed="appStore.sidebarCollapsed"
      :collapsed-width="64"
      :width="220"
      collapse-mode="width"
      show-trigger="bar"
      style="background: var(--n-sidebar-color)"
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

    <!-- 主内容区 -->
    <n-layout style="position: relative">
      <n-layout-content
        style="padding: 24px 24px 80px"
        content-style="background: var(--n-color);"
      >
        <!-- 内容区：最大化窗口下充分利用宽度（首页内部自适应两栏） -->
        <div class="page-container">
          <router-view />
        </div>
      </n-layout-content>

      <!-- 底部播放器（播放页为全屏覆盖，故隐藏） -->
      <PlayerBar v-if="!isPlayerRoute" />
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
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
import { getLoginStatus, type MusicSource } from '@/api'

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

onMounted(refreshProviderStatus)

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
/* ---------- 主内容区 ---------- */
.page-container {
  max-width: 1800px;
  margin: 0 auto;
  width: 100%;
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
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

/* 收起时的页签徽章 */
.logo-tab {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  border-radius: 12px 12px 12px 4px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  font-weight: 800;
  letter-spacing: 0.5px;
  box-shadow: 0 4px 14px rgba(102, 126, 234, 0.45);
  transition: transform 0.2s, box-shadow 0.2s;
}

.logo-tab:hover {
  transform: scale(1.06);
  box-shadow: 0 6px 18px rgba(102, 126, 234, 0.55);
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
  background: linear-gradient(135deg, #31c27c 0%, #1fa96b 100%);
}

/* 网易云：品牌红 */
.tp-card.netease .tp-icon {
  background: linear-gradient(135deg, #d43c33 0%, #b0322a 100%);
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
  background: linear-gradient(135deg, #31c27c 0%, #1fa96b 100%);
}
</style>
