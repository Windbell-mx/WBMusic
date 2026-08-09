<template>
  <div class="playlist-page">
    <!-- 页头 -->
    <div class="page-header">
      <div class="header-left">
        <n-h1 class="header-title">我的歌单</n-h1>
        <n-text class="header-sub">
          <template v-if="loading">正在加载歌单...</template>
          <template v-else>
            共 {{ playlists.length }} 个歌单 · {{ totalTracks }} 首歌曲
            <template v-if="userNickname"> · {{ userNickname }}</template>
          </template>
        </n-text>
      </div>
      <div class="header-right">
        <n-input
          v-model:value="searchQuery"
          placeholder="搜索歌单..."
          clearable
          size="medium"
          class="search-input"
        >
          <template #prefix>
            <n-icon :component="Search" />
          </template>
        </n-input>
        <n-button type="primary" @click="openCreateModal">
          <template #icon>
            <n-icon :component="Add" />
          </template>
          新建歌单
        </n-button>
      </div>
    </div>

    <!-- 平台筛选 -->
    <div class="category-bar">
      <div
        v-for="cat in categories"
        :key="cat.key"
        class="category-item"
        :class="{ active: activeCategory === cat.key }"
        @click="switchCategory(cat.key)"
      >
        {{ cat.label }}
        <span class="cat-count">{{ categoryCount(cat.key) }}</span>
      </div>
    </div>

    <!-- 未登录提示 -->
    <n-alert
      v-if="!loading && !anyLoggedIn && playlists.length === 0"
      type="warning"
      :show-icon="false"
      style="margin-bottom: 24px"
    >
      <template #default>
        <n-space align="center" justify="space-between">
          <span>尚未登录音乐平台，登录后可同步你的真实歌单（点击左侧「第三方接入」登录）</span>
          <n-button size="small" secondary type="warning" @click="goSettings">
            去登录
          </n-button>
        </n-space>
      </template>
    </n-alert>

    <!-- 歌单网格（3 列布局，超窄屏回退 1 列） -->
    <n-grid cols="s:1 m:3 l:3 xl:3 2xl:3" x-gap="20" y-gap="28" responsive="screen">
      <n-gi v-for="playlist in filteredPlaylists" :key="playlist.id">
        <div class="playlist-card" @click="selectPlaylist(playlist)">
          <div class="card-cover">
            <n-image
              :src="playlist.cover_url || fallbackCover(playlist)"
              :alt="playlist.name"
              class="cover-img"
              object-fit="cover"
              :preview-disabled="true"
            />
            <!-- 平台角标 -->
            <span class="source-badge" :class="`source-${playlist.source}`">
              {{ sourceLabel(playlist.source) }}
            </span>
            <!-- 播放量角标 -->
            <span class="play-count" v-if="playlist.play_count > 0">
              <n-icon :component="Headset" size="12" />
              {{ formatPlays(playlist.play_count) }}
            </span>
            <!-- 悬停播放按钮 -->
            <div class="cover-mask">
              <n-button
                circle
                class="cover-play-btn"
                @click.stop="playPlaylist(playlist)"
              >
                <template #icon>
                  <n-icon :component="Play" size="22" />
                </template>
              </n-button>
            </div>
          </div>
          <div class="card-body">
            <div class="card-title" :title="playlist.name">{{ playlist.name }}</div>
            <div class="card-desc" :title="playlist.description || ''">
              {{ playlist.description || '暂无描述' }}
            </div>
            <div class="card-footer">
              <n-tag size="small" :bordered="false" type="info" round>
                {{ playlist.track_count }} 首
              </n-tag>
            </div>
          </div>
        </div>
      </n-gi>
    </n-grid>

    <n-empty
      v-if="!loading && filteredPlaylists.length === 0"
      :description="emptyDescription"
      style="margin-top: 64px"
    >
      <template #extra>
        <n-space>
          <n-button size="small" secondary @click="resetFilters">清除筛选</n-button>
          <n-button v-if="!anyLoggedIn" size="small" secondary @click="goSettings">
            去登录
          </n-button>
        </n-space>
      </template>
    </n-empty>

    <!-- 新建歌单对话框 -->
    <n-modal
      v-model:show="showCreateModal"
      preset="card"
      title="新建歌单"
      style="width: 440px"
      :mask-closable="false"
    >
      <n-form ref="formRef" :model="formModel" :rules="formRules" label-placement="top">
        <n-form-item label="歌单名称" path="name">
          <n-input
            v-model:value="formModel.name"
            placeholder="输入歌单名称"
            maxlength="20"
            show-count
          />
        </n-form-item>
        <n-form-item label="歌单描述" path="description">
          <n-input
            v-model:value="formModel.description"
            placeholder="介绍一下这个歌单（可选）"
            type="textarea"
            :rows="3"
            maxlength="60"
            show-count
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button secondary @click="showCreateModal = false">取消</n-button>
          <n-button type="primary" @click="handleCreate">创建</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  NIcon,
  NButton,
  NInput,
  NImage,
  NTag,
  NGrid,
  NGi,
  NEmpty,
  NModal,
  NForm,
  NFormItem,
  NSpace,
  NAlert,
  type FormInst,
  type FormRules,
  useMessage,
} from 'naive-ui'
import {
  Play,
  Add,
  Search,
  Headset,
} from '@vicons/ionicons5'
import {
  getUserPlaylists,
  getLoginStatus,
  type Playlist as ApiPlaylist,
  type MusicSource,
} from '@/api'

const router = useRouter()
const message = useMessage()

const categories = [
  { key: 'all', label: '全部' },
  { key: 'netease', label: '网易云' },
  { key: 'qq_music', label: 'QQ 音乐' },
]

const playlists = ref<ApiPlaylist[]>([])
const loading = ref(false)
const anyLoggedIn = ref(false)
const userNickname = ref('')

const searchQuery = ref('')
const activeCategory = ref('all')

const totalTracks = computed(() =>
  playlists.value.reduce((sum, p) => sum + p.track_count, 0)
)

const emptyDescription = computed(() => {
  if (anyLoggedIn.value) return '当前平台没有歌单'
  return '尚未登录音乐平台，登录后可同步真实歌单'
})

const filteredPlaylists = computed(() => {
  const keyword = searchQuery.value.trim().toLowerCase()
  return playlists.value.filter((p) => {
    const matchCategory =
      activeCategory.value === 'all' || p.source === activeCategory.value
    const matchKeyword =
      !keyword ||
      p.name.toLowerCase().includes(keyword) ||
      (p.description || '').toLowerCase().includes(keyword)
    return matchCategory && matchKeyword
  })
})

function categoryCount(key: string) {
  if (key === 'all') return playlists.value.length
  return playlists.value.filter((p) => p.source === key).length
}

function switchCategory(key: string) {
  activeCategory.value = key
}

function formatPlays(count: number) {
  if (count >= 100000000) return (count / 100000000).toFixed(1) + '亿'
  if (count >= 10000) return (count / 10000).toFixed(1) + '万'
  return String(count)
}

function sourceLabel(source: MusicSource) {
  return source === 'netease' ? '网易云' : 'QQ 音乐'
}

function fallbackCover(playlist: ApiPlaylist) {
  return `https://picsum.photos/seed/${playlist.source}-${playlist.id}/400/400`
}

function selectPlaylist(playlist: ApiPlaylist) {
  router.push({
    name: 'PlaylistDetail',
    params: { id: playlist.id },
    query: { source: playlist.source },
  })
}

function playPlaylist(playlist: ApiPlaylist) {
  router.push({
    name: 'Player',
    query: { playlistId: playlist.id, source: playlist.source },
  })
}

function goSettings() {
  router.push({ name: 'Settings' })
}

function resetFilters() {
  searchQuery.value = ''
  activeCategory.value = 'all'
}

async function loadPlaylists() {
  loading.value = true
  try {
    // 登录状态
    const statuses = await getLoginStatus()
    anyLoggedIn.value =
      statuses.netease?.logged_in || statuses.qq_music?.logged_in || false
    userNickname.value =
      statuses.netease?.nickname || statuses.qq_music?.nickname || ''

    // 加载已登录平台的歌单
    const all: ApiPlaylist[] = []
    if (statuses.netease?.logged_in) {
      try {
        const list = await getUserPlaylists('netease')
        all.push(...list)
      } catch (e) {
        console.warn('加载网易云歌单失败:', e)
      }
    }
    if (statuses.qq_music?.logged_in) {
      try {
        const list = await getUserPlaylists('qq_music')
        all.push(...list)
      } catch (e) {
        console.warn('加载 QQ 音乐歌单失败:', e)
      }
    }
    playlists.value = all
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadPlaylists()
  // 窗口重新聚焦时自动刷新（如从登录弹窗/设置页返回后，或调试时切回应用）
  window.addEventListener('focus', loadPlaylists)
})

/* ---------- 新建歌单（本地占位，标注来源） ---------- */
const showCreateModal = ref(false)
const formRef = ref<FormInst | null>(null)
const formModel = ref({
  name: '',
  description: '',
})

const formRules: FormRules = {
  name: {
    required: true,
    message: '请输入歌单名称',
    trigger: ['input', 'blur'],
  },
}

function openCreateModal() {
  formModel.value = { name: '', description: '' }
  showCreateModal.value = true
}

function handleCreate() {
  formRef.value?.validate((errors) => {
    if (errors) return
    playlists.value.unshift({
      id: `local-${Date.now()}`,
      name: formModel.value.name,
      description: formModel.value.description || '暂无描述',
      cover_url: `https://picsum.photos/seed/playlist${Date.now()}/400/400`,
      track_count: 0,
      play_count: 0,
      source: 'netease',
    })
    showCreateModal.value = false
    activeCategory.value = 'all'
    message.success(`歌单「${formModel.value.name}」创建成功（本地）`)
  })
}
</script>

<style scoped>
.playlist-page {
  padding: 8px;
}

/* ---------- 页头 ---------- */
.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  flex-wrap: wrap;
  margin-bottom: 28px;
}

.header-title {
  margin: 0;
  font-size: 26px;
  font-weight: 700;
}

.header-sub {
  font-size: 13px;
  color: var(--n-text-color-3);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.search-input {
  width: 220px;
}

/* ---------- 分类筛选 ---------- */
.category-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  border-bottom: 1px solid var(--n-border-color);
  margin-bottom: 24px;
}

.category-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  user-select: none;
  color: var(--n-text-color-3);
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  transition: color 0.2s;
}

.category-item:hover {
  color: var(--n-text-color);
}

.category-item.active {
  color: var(--n-primary-color);
  border-bottom-color: var(--n-primary-color);
  font-weight: 600;
}

.cat-count {
  font-size: 11px;
  opacity: 0.6;
  font-weight: 400;
}

/* ---------- 歌单卡片 ---------- */
.playlist-card {
  cursor: pointer;
  transition: transform 0.2s ease;
}

.playlist-card:hover {
  transform: translateY(-3px);
}

.card-cover {
  position: relative;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  transition: box-shadow 0.2s ease;
}

.playlist-card:hover .card-cover {
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.12);
}

.cover-img {
  display: block;
  width: 100%;
  height: 180px;
  border-radius: 10px;
}

.play-count {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 999px;
  font-size: 11px;
  color: #fff;
  background: rgba(0, 0, 0, 0.45);
}

.source-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  padding: 3px 8px;
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

.cover-mask {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  padding: 12px;
  background: linear-gradient(to top, rgba(0, 0, 0, 0.3), transparent 45%);
  opacity: 0;
  transition: opacity 0.2s ease;
}

.playlist-card:hover .cover-mask {
  opacity: 1;
}

.cover-play-btn {
  width: 40px;
  height: 40px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  transition: transform 0.2s ease;
}

.cover-play-btn:hover {
  transform: scale(1.08);
}

.card-body {
  padding: 10px 2px 0;
}

.card-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--n-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 2px;
}

.card-desc {
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 10px;
}

.card-footer {
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
