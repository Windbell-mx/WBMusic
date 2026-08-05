<template>
  <div class="home-page">
    <div class="home-layout">
      <!-- ===== 主内容区 ===== -->
      <main class="home-main">
        <!-- 欢迎区 -->
        <div class="welcome">
          <h1 class="welcome-title">欢迎使用 WBMusic</h1>
          <p class="welcome-sub">
            {{ anyLoggedIn ? '已连接音乐平台，开始享受你的音乐吧' : '登录音乐平台后，同步你的真实歌单与收藏' }}
          </p>
        </div>

        <!-- 未登录：登录引导 -->
        <div v-if="!anyLoggedIn" class="guide-card">
          <div class="guide-icon">
            <n-icon :component="MusicalNotes" size="40" />
          </div>
          <h2 class="guide-title">连接你的音乐账号</h2>
          <p class="guide-desc">
            WBMusic 支持网易云音乐与 QQ 音乐。
            登录后即可同步你的歌单、收藏与播放记录。
          </p>
          <n-space justify="center">
            <n-button type="primary" size="large" @click="goSettings">
              去登录
            </n-button>
          </n-space>
        </div>

        <!-- 已登录：引导入口 -->
        <div v-else class="guide-card logged-in">
          <div class="guide-icon">
            <n-icon :component="CheckmarkCircle" size="40" />
          </div>
          <h2 class="guide-title">已连接</h2>
          <p class="guide-desc">
            已登录平台：{{ connectedLabels.join('、') }}。前往歌单页查看你的音乐。
          </p>
          <n-space justify="center">
            <n-button type="primary" size="large" @click="goPlaylist">
              查看我的歌单
            </n-button>
            <n-button size="large" secondary @click="goSearch">搜索音乐</n-button>
          </n-space>
        </div>
      </main>

      <!-- ===== 右侧边栏 ===== -->
      <aside class="home-aside">
        <!-- 登录卡片 -->
        <div class="aside-card">
          <div class="aside-head"><span class="aside-title">音乐账号</span></div>
          <div class="login-provider" @click="openLogin('qq_music', 'QQ 音乐')">
            <span class="login-dot qq"></span>
            <span class="login-name">QQ 音乐</span>
            <span class="login-status" :class="{ on: qqLoggedIn }">{{ qqLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
          <div class="login-provider" @click="openLogin('netease', '网易云音乐')">
            <span class="login-dot netease"></span>
            <span class="login-name">网易云音乐</span>
            <span class="login-status" :class="{ on: neteaseLoggedIn }">{{ neteaseLoggedIn ? '已登录' : '未登录' }}</span>
          </div>
        </div>

        <!-- 快捷入口 -->
        <div class="aside-card">
          <div class="aside-head"><span class="aside-title">快捷入口</span></div>
          <div class="quick-entry" @click="goPlaylist">
            <n-icon :component="List" size="18" />
            <span>我的歌单</span>
          </div>
          <div class="quick-entry" @click="goSearch">
            <n-icon :component="Search" size="18" />
            <span>搜索音乐</span>
          </div>
          <div class="quick-entry" @click="goSettings">
            <n-icon :component="Settings" size="18" />
            <span>设置</span>
          </div>
        </div>
      </aside>
    </div>

    <!-- 登录弹窗 -->
    <LoginModal v-model:show="showLoginModal" :provider="loginTarget" @changed="refreshLogin" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon, NButton, NSpace } from 'naive-ui'
import {
  MusicalNotes,
  CheckmarkCircle,
  List,
  Search,
  Settings,
} from '@vicons/ionicons5'
import type { MusicSource } from '@/api'
import { getLoginStatus } from '@/api'
import LoginModal from '@/components/LoginModal.vue'

const router = useRouter()

/* ---------- 登录状态 ---------- */

const qqLoggedIn = ref(false)
const neteaseLoggedIn = ref(false)

const anyLoggedIn = computed(() => qqLoggedIn.value || neteaseLoggedIn.value)

const connectedLabels = computed(() => {
  const labels: string[] = []
  if (qqLoggedIn.value) labels.push('QQ 音乐')
  if (neteaseLoggedIn.value) labels.push('网易云音乐')
  return labels
})

async function refreshLogin() {
  try {
    const s = await getLoginStatus()
    qqLoggedIn.value = s.qq_music?.logged_in ?? false
    neteaseLoggedIn.value = s.netease?.logged_in ?? false
  } catch {
    /* 浏览器降级环境忽略 */
  }
}
onMounted(refreshLogin)

/* ---------- 登录弹窗 ---------- */

const showLoginModal = ref(false)
const loginTarget = ref<{ key: MusicSource; label: string } | null>(null)

function openLogin(key: MusicSource, label: string) {
  loginTarget.value = { key, label }
  showLoginModal.value = true
}

/* ---------- 跳转 ---------- */

function goPlaylist() {
  router.push({ name: 'Playlist' })
}

function goSearch() {
  router.push({ name: 'Search' })
}

function goSettings() {
  router.push({ name: 'Settings' })
}
</script>

<style scoped>
.home-page {
  padding-bottom: 24px;
}

/* ===== 两栏布局 ===== */
.home-layout {
  display: flex;
  gap: 28px;
  align-items: flex-start;
}

.home-main {
  flex: 1;
  min-width: 0;
}

.home-aside {
  width: 300px;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* 窄屏时收起右侧边栏 */
@media (max-width: 1360px) {
  .home-aside {
    display: none;
  }
}

/* ===== 欢迎区 ===== */
.welcome {
  padding: 40px 0 28px;
}

.welcome-title {
  margin: 0 0 8px;
  font-size: 32px;
  font-weight: 800;
  background: linear-gradient(120deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.welcome-sub {
  margin: 0;
  font-size: 15px;
  color: var(--n-text-color-3);
}

/* ===== 引导卡片 ===== */
.guide-card {
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 16px;
  padding: 56px 32px;
  text-align: center;
}

.guide-icon {
  color: #667eea;
  margin-bottom: 16px;
}

.guide-title {
  margin: 0 0 10px;
  font-size: 22px;
  font-weight: 700;
  color: var(--n-text-color);
}

.guide-desc {
  margin: 0 auto 24px;
  max-width: 440px;
  font-size: 14px;
  line-height: 1.7;
  color: var(--n-text-color-3);
}

/* ===== 右侧边栏卡片 ===== */
.aside-card {
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 12px;
  padding: 16px;
}

.aside-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.aside-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--n-text-color);
}

/* 登录卡片 */
.login-provider {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.15s;
}

.login-provider:hover {
  background: var(--n-color-2);
}

.login-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.login-dot.qq {
  background: #31c27c;
}

.login-dot.netease {
  background: #d43c33;
}

.login-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--n-text-color);
}

.login-status {
  margin-left: auto;
  font-size: 12px;
  color: var(--n-text-color-3);
}

.login-status.on {
  color: #31c27c;
  font-weight: 600;
}

/* 快捷入口 */
.quick-entry {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px;
  border-radius: 10px;
  cursor: pointer;
  color: var(--n-text-color-2);
  transition: background 0.15s, color 0.15s;
}

.quick-entry:hover {
  background: var(--n-color-2);
  color: var(--n-text-color);
}
</style>
