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

        <!-- 推荐歌单（QQ 音乐登录后为个性化推荐，未登录回退热门） -->
        <section class="recommend-section">
          <div class="recommend-head">
            <h2 class="recommend-title">发现音乐</h2>
            <div class="recommend-tabs">
              <div
                v-for="tab in platformTabs"
                :key="tab.key"
                class="recommend-tab"
                :class="{ active: activeRecTab === tab.key }"
                @click="switchPlatform(tab.key)"
              >
                {{ tab.label }}
              </div>
            </div>
          </div>

          <!-- 二级分类标签 -->
          <div class="category-tabs">
            <div
              v-for="cat in currentCategories"
              :key="cat.key"
              class="category-tab"
              :class="{ active: activeCategory === cat.key }"
              @click="switchCategory(cat.key)"
            >
              {{ cat.label }}
            </div>
          </div>

          <n-spin :show="recLoading">
            <div v-if="!recLoading && recError" class="rec-state">
              {{ recError }}<span class="rec-retry" @click="loadRecommend(true)">点击重试</span>
            </div>
            <div v-else-if="!recLoading && !recPlaylists.length" class="rec-state">
              暂无内容
            </div>
            <div v-else class="recommend-row">
              <div
                v-for="p in recPlaylists"
                :key="p.id"
                class="rec-card"
                @click="openPlaylist(p)"
              >
                <div class="rec-cover">
                  <n-image
                    :src="p.cover_url || fallbackCover(p)"
                    :alt="p.name"
                    class="rec-img"
                    object-fit="cover"
                    :preview-disabled="true"
                  />
                  <span class="source-badge" :class="`source-${p.source}`">
                    {{ sourceLabel(p.source) }}
                  </span>
                  <span class="play-count" v-if="p.play_count > 0">
                    <n-icon :component="Headset" size="11" />
                    {{ formatPlays(p.play_count) }}
                  </span>
                </div>
                <div class="rec-name" :title="p.name">{{ p.name }}</div>
                <div class="rec-desc" :title="p.description || ''">
                  {{ p.description || '暂无描述' }}
                </div>
              </div>
            </div>
          </n-spin>
        </section>

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
import { NIcon, NButton, NSpace, NSpin } from 'naive-ui'
import {
  MusicalNotes,
  CheckmarkCircle,
  List,
  Search,
  Settings,
  Headset,
} from '@vicons/ionicons5'
import type { MusicSource, HomeCategory } from '@/api'
import {
  getLoginStatus,
  getCategoryPlaylists,
  type Playlist as ApiPlaylist,
} from '@/api'
import LoginModal from '@/components/LoginModal.vue'
import { getCached, cachePeek, CACHE_TTL } from '@/utils/cache'

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

/* ---------- 发现音乐：平台 + 分类二级标签 ---------- */

// 本地存储键：记住用户停留在哪个平台/分类，从歌单详情返回时保持状态
const STORAGE_TAB = 'wbmusic.homeTab'
const STORAGE_CATEGORY = 'wbmusic.homeCategory'

interface PlatformTab {
  key: MusicSource
  label: string
}

interface CategoryTab {
  key: HomeCategory
  label: string
}

const platformTabs: PlatformTab[] = [
  { key: 'netease', label: '网易云音乐' },
  { key: 'qq_music', label: 'QQ 音乐' },
]

// 各平台支持的二级分类
const platformCategories: Record<MusicSource, CategoryTab[]> = {
  netease: [
    { key: 'daily', label: '每日推荐' },
    { key: 'featured', label: '为你推荐' },
    { key: 'hot', label: '热歌榜' },
  ],
  qq_music: [
    { key: 'rec', label: '为你推荐' },
    { key: 'hot', label: '排行榜' },
  ],
}

// 恢复持久化的平台标签（默认网易云）
const storedTab = localStorage.getItem(STORAGE_TAB) as MusicSource | null
const activeRecTab = ref<MusicSource>(
  storedTab === 'netease' || storedTab === 'qq_music' ? storedTab : 'netease',
)

// 恢复持久化的分类标签（默认该平台第一个分类）
const storedCategory = localStorage.getItem(STORAGE_CATEGORY) as HomeCategory | null
const activeCategory = ref<HomeCategory>(
  storedCategory && platformCategories[activeRecTab.value].some((c) => c.key === storedCategory)
    ? storedCategory
    : platformCategories[activeRecTab.value][0].key,
)

const currentCategories = computed(() => platformCategories[activeRecTab.value])

const recPlaylists = ref<ApiPlaylist[]>([])
const recLoading = ref(false)
const recError = ref('')

/** 首页发现音乐缓存键：按平台 + 分类区分 */
function recCacheKey(): string {
  return `home:${activeRecTab.value}:${activeCategory.value}`
}

async function loadRecommend(force = false) {
  const key = recCacheKey()
  recError.value = ''
  if (!force) {
    // 内存缓存命中：直接秒显，不显示 loading
    const hit = cachePeek<ApiPlaylist[]>(key, CACHE_TTL.home)
    if (hit) {
      recPlaylists.value = hit
      return
    }
  }
  recLoading.value = true
  try {
    recPlaylists.value = await getCached(
      key,
      () => getCategoryPlaylists(activeRecTab.value, activeCategory.value, 10),
      CACHE_TTL.home,
      force,
    )
  } catch (e) {
    recError.value = '内容加载失败'
    console.warn('加载分类歌单失败:', e)
  } finally {
    recLoading.value = false
  }
}

function switchPlatform(key: MusicSource) {
  if (key === activeRecTab.value) return
  activeRecTab.value = key
  // 切换平台时，分类重置为该平台默认分类
  activeCategory.value = platformCategories[key][0].key
  localStorage.setItem(STORAGE_TAB, key)
  localStorage.setItem(STORAGE_CATEGORY, activeCategory.value)
  loadRecommend()
}

function switchCategory(key: HomeCategory) {
  if (key === activeCategory.value) return
  activeCategory.value = key
  localStorage.setItem(STORAGE_CATEGORY, key)
  loadRecommend()
}

// 进入页面时先恢复持久化的状态再加载
onMounted(() => {
  localStorage.setItem(STORAGE_TAB, activeRecTab.value)
  localStorage.setItem(STORAGE_CATEGORY, activeCategory.value)
  loadRecommend()
})

function formatPlays(count: number) {
  if (count >= 100000000) return (count / 100000000).toFixed(1) + '亿'
  if (count >= 10000) return (count / 10000).toFixed(1) + '万'
  return String(count)
}

function sourceLabel(source: MusicSource) {
  return source === 'netease' ? '网易云' : 'QQ 音乐'
}

function fallbackCover(p: ApiPlaylist) {
  return `https://picsum.photos/seed/${p.source}-${p.id}/400/400`
}

function openPlaylist(p: ApiPlaylist) {
  router.push({
    name: 'PlaylistDetail',
    params: { id: p.id },
    query: { source: p.source },
  })
}

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
  color: var(--accent);
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
  color: var(--accent);
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

/* ===== 推荐歌单 ===== */
.recommend-section {
  margin-bottom: 28px;
}

.recommend-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.recommend-title {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  color: var(--n-text-color);
}

.recommend-tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  border-radius: 999px;
  /* 浅色玻璃：纯白半透明底 + 顶部高光 + 柔和投影，仍通透。
     毛玻璃在性能模式（html.perf-mode）下由全局规则关闭 */
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(16px) saturate(1.8);
  -webkit-backdrop-filter: blur(16px) saturate(1.8);
  border: 1px solid rgba(255, 255, 255, 0.65);
  box-shadow:
    0 6px 20px rgba(31, 38, 135, 0.1),
    0 2px 6px rgba(0, 0, 0, 0.06),
    inset 0 1px 0 rgba(255, 255, 255, 0.9),
    inset 0 -1px 0 rgba(255, 255, 255, 0.25);
}

html.dark .recommend-tabs {
  /* 暗色无底色：仅保留模糊玻璃感 + 细边框，完全透出底下内容 */
  background: transparent;
  backdrop-filter: blur(16px) saturate(1.8);
  -webkit-backdrop-filter: blur(16px) saturate(1.8);
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.recommend-tab {
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  user-select: none;
  color: var(--n-text-color-3);
  transition: color 0.2s, background 0.2s, box-shadow 0.2s;
}

.recommend-tab:hover {
  color: var(--n-text-color);
  background: rgba(255, 255, 255, 0.55);
}

html.dark .recommend-tab:hover {
  background: transparent;
}

.recommend-tab.active {
  color: var(--accent);
  /* 激活态：透明毛玻璃胶囊（透出底下 + 模糊），主题色 + 高光 */
  background: rgba(255, 255, 255, 0.55);
  backdrop-filter: blur(12px) saturate(1.6);
  -webkit-backdrop-filter: blur(12px) saturate(1.6);
  box-shadow:
    0 2px 8px rgba(31, 38, 135, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.9);
  font-weight: 600;
}

html.dark .recommend-tab.active {
  /* 暗色激活：完全透明毛玻璃（无底色），主题色文字 + 加粗 + 极淡主题色描边 */
  color: var(--accent-light);
  background: transparent;
  backdrop-filter: blur(12px) saturate(1.6);
  -webkit-backdrop-filter: blur(12px) saturate(1.6);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-light) 30%, transparent);
}

/* 二级分类标签 */
.category-tabs {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.category-tab {
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  user-select: none;
  color: var(--n-text-color-3);
  background: rgba(255, 255, 255, 0.35);
  transition: color 0.2s, background 0.2s;
}

html.dark .category-tab {
  background: rgba(255, 255, 255, 0.06);
}

.category-tab:hover {
  color: var(--n-text-color);
}

.category-tab.active {
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  backdrop-filter: blur(12px) saturate(1.6);
  -webkit-backdrop-filter: blur(12px) saturate(1.6);
  font-weight: 600;
}

html.dark .category-tab.active {
  color: var(--accent-light);
  background: transparent;
  backdrop-filter: blur(12px) saturate(1.6);
  -webkit-backdrop-filter: blur(12px) saturate(1.6);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-light) 30%, transparent);
}

.recommend-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 20px;
}

.rec-card {
  cursor: pointer;
  transition: transform 0.2s ease;
}

.rec-card:hover {
  transform: translateY(-3px);
}

.rec-cover {
  position: relative;
  border-radius: 10px;
  overflow: hidden;
  aspect-ratio: 1 / 1;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  transition: box-shadow 0.2s ease;
}

.rec-card:hover .rec-cover {
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.12);
}

.rec-img {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: 10px;
}

.source-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  color: #fff;
  background: rgba(0, 0, 0, 0.45);
}

.source-netease {
  background: rgba(194, 59, 59, 0.85);
}

.source-qq_music {
  background: rgba(22, 113, 255, 0.85);
}

.play-count {
  position: absolute;
  bottom: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  color: #fff;
  background: rgba(0, 0, 0, 0.45);
}

.rec-name {
  margin-top: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--n-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rec-desc {
  margin-top: 2px;
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rec-state {
  padding: 48px 0;
  text-align: center;
  font-size: 14px;
  color: var(--n-text-color-3);
}

.rec-retry {
  margin-left: 8px;
  color: var(--n-primary-color);
  cursor: pointer;
}
</style>
