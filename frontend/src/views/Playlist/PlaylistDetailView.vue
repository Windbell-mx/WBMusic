<template>
  <n-layout>
    <n-layout-header style="padding: 16px; background: var(--n-color)">
      <n-space justify="space-between" align="center">
        <n-space>
          <n-button text @click="router.back()">
            <n-icon :component="ArrowBack" size="24" />
          </n-button>
          <n-h2 style="margin: 0">{{ playlist?.name }}</n-h2>
        </n-space>
        <n-space>
          <n-button type="primary" @click="playAll">
            <template #icon>
              <n-icon :component="Play" />
            </template>
            播放全部
          </n-button>
          <n-button @click="shufflePlay">
            <template #icon>
              <n-icon :component="Shuffle" />
            </template>
            随机播放
          </n-button>
        </n-space>
      </n-space>
    </n-layout-header>

    <n-layout-content style="padding: 16px">
      <!-- 加载中 -->
      <div v-if="loading" class="state-wrap">
        <n-spin size="large" />
        <n-text depth="3" style="margin-top: 12px">正在加载歌单...</n-text>
      </div>

      <!-- 加载失败 -->
      <div v-else-if="!playlist" class="state-wrap">
        <n-empty :description="needLogin ? '需要登录' : '歌单加载失败'">
          <template #extra>
            <n-text depth="3" style="display: block; margin-bottom: 16px; max-width: 480px">
              {{ errorMsg }}
            </n-text>
            <n-button v-if="needLogin" type="primary" @click="openLogin">
              去登录 {{ sourceLabel }}
            </n-button>
          </template>
        </n-empty>
      </div>

      <template v-else>
        <div class="detail-header">
          <n-image
            class="detail-cover"
            :src="playlist.cover"
            :alt="playlist.name"
            object-fit="cover"
          />
          <div class="detail-info">
            <h1 class="detail-title">{{ playlist.name }}</h1>
            <div class="detail-meta">
              <n-tag size="small" round>{{ playlist.trackCount }} 首歌曲</n-tag>
              <n-tag size="small" round>{{ playlist.tracks[0]?.source === 'netease' ? '网易云' : 'QQ 音乐' }}</n-tag>
            </div>
            <p class="detail-desc">{{ playlist.description }}</p>
            <div class="detail-actions">
              <n-button type="primary" size="large" round @click="playAll">
                <template #icon>
                  <n-icon :component="Play" />
                </template>
                播放
              </n-button>
              <n-button size="large" round @click="shufflePlay">
                <template #icon>
                  <n-icon :component="Shuffle" />
                </template>
                随机播放
              </n-button>
            </div>
          </div>
        </div>

        <n-divider />

        <n-data-table
          :columns="columns"
          :data="playlist.tracks || []"
          :scroll-x="300"
          striped
        />
      </template>
    </n-layout-content>
  </n-layout>

  <!-- 需要登录时弹出登录框 -->
  <LoginModal v-model:show="showLoginModal" :provider="loginTarget" @changed="onLoginChanged" />
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NIcon, NButton, NSpin, NEmpty, NSpace, NDropdown, NEllipsis, createDiscreteApi } from 'naive-ui'
import {
  Play,
  Shuffle,
  ArrowBack,
  Mic,
  Time,
  Star,
  EllipsisHorizontal,
} from '@vicons/ionicons5'
import type { DataTableColumns } from 'naive-ui'
import { usePlayerStore } from '@/stores/player'
import LoginModal from '@/components/LoginModal.vue'
import {
  getPlaylistDetail,
  likeTrack,
  isTauri,
  type Track as ApiTrack,
  type MusicSource,
} from '@/api'

const router = useRouter()
const route = useRoute()
const player = usePlayerStore()

/** 全局消息提示 */
const { message } = createDiscreteApi(['message'])

/** 当前详情页的播放源（默认从 query 读取） */
const detailSource = ref<MusicSource>((route.query.source as MusicSource) || 'netease')

/* ---------- 登录引导 ---------- */

const showLoginModal = ref(false)
const needLogin = ref(false)

const loginTarget = computed(() => ({
  key: detailSource.value,
  label: detailSource.value === 'qq_music' ? 'QQ 音乐' : '网易云音乐',
}))

const sourceLabel = computed(() => loginTarget.value.label)

function openLogin() {
  showLoginModal.value = true
}

function onLoginChanged() {
  // 登录成功后重新加载歌单详情
  showLoginModal.value = false
  loadDetail()
}

interface Track {
  id: string
  title: string
  artist: string
  album: string
  duration: string
  isLiked: boolean
  source: MusicSource
  coverUrl?: string | null
}

interface Playlist {
  id: string
  name: string
  description: string
  cover: string
  trackCount: number
  tracks: Track[]
}

const playlist = ref<Playlist | null>(null)
const loading = ref(true)
const errorMsg = ref('')

// 本地曲目 → 播放器 store 曲目
function toApiTrack(track: Track): ApiTrack {
  // 本地 duration 是 "mm:ss" 字符串，转回秒数
  const [m, s] = (track.duration || '').split(':').map(Number)
  const duration = !Number.isNaN(m) ? m * 60 + (Number.isNaN(s) ? 0 : s) : undefined
  return {
    id: track.id,
    title: track.title,
    artist: track.artist,
    album: track.album,
    cover_url: track.coverUrl,
    duration,
    source: track.source,
  }
}

/** 秒 → mm:ss */
function formatDuration(sec: number | undefined): string {
  if (!sec || sec <= 0) return '--:--'
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  return `${m}:${String(s).padStart(2, '0')}`
}

function fallbackCover(id: string) {
  return `https://picsum.photos/seed/playlist-track-${id}/200/200`
}

/** naive-ui 运行时支持 flexGrow（类型定义缺失，这里扩展类型） */
type DetailColumn = DataTableColumns<Track>[number] & { flexGrow?: number }

const columns: DetailColumn[] = [
  {
    title: '#',
    key: 'index',
    width: 44,
    render: (_row, index) => index + 1,
  },
  {
    title: '歌曲',
    key: 'title',
    flexGrow: 1,
    render: (row) => {
      return h(
        'div',
        {
          style: 'display: flex; align-items: center; gap: 6px; min-width: 0; width: 100%',
        },
        [
          h(NIcon, { component: Mic, style: 'flex-shrink: 0' }),
          h(
            'a',
            {
              style:
                'flex: 1; min-width: 0; overflow: hidden; cursor: pointer; color: inherit; text-decoration: none; display: block',
              onClick: () => playSingle(row),
            },
            h(
              NEllipsis,
              {
                tooltip: true,
                style: 'max-width: 100%',
              },
              { default: () => row.title },
            ),
          ),
        ],
      )
    },
  },
  {
    title: '歌手',
    key: 'artist',
    flexGrow: 1,
    ellipsis: {
      tooltip: true,
    },
  },
  {
    title: '专辑',
    key: 'album',
    flexGrow: 1,
    ellipsis: {
      tooltip: true,
    },
  },
  {
    title: () => h('span', { style: 'padding-left: 28px' }, '时长'),
    key: 'duration',
    flexGrow: 1,
    render: (row) => {
      return h(
        NSpace,
        { align: 'center', style: 'padding-left: 28px' },
        {
          default: () => [
            h(NIcon, { component: Time, size: 14 }),
            h('span', null, row.duration),
          ],
        },
      )
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 72,
    render: (row) => {
      const menuOptions = [
        { label: '播放', key: 'play' },
        { label: '下一首播放', key: 'play-next' },
        { label: row.isLiked ? '取消收藏' : '收藏', key: 'like' },
      ]
      return h(
        NSpace,
        null,
        {
          default: () => [
            h(
              NButton,
              {
                text: true,
                type: row.isLiked ? 'primary' : undefined,
                onClick: () => toggleLike(row),
              },
              { default: () => h(NIcon, { component: Star }) }
            ),
            h(
              NDropdown,
              {
                options: menuOptions,
                trigger: 'click',
                onSelect: (key: string) => onMenuSelect(key, row),
              },
              {
                default: () =>
                  h(
                    NButton,
                    { text: true },
                    { default: () => h(NIcon, { component: EllipsisHorizontal }) }
                  ),
              }
            ),
          ],
        },
      )
    },
  },
]

async function toggleLike(track: Track) {
  const target = !track.isLiked
  track.isLiked = target
  // 浏览器环境直接模拟成功
  if (!isTauri) {
    message.success(target ? '已收藏（演示）' : '已取消收藏（演示）')
    return
  }
  try {
    await likeTrack(track.source, track.id, target)
    message.success(target ? '已收藏到默认喜欢歌单' : '已取消收藏')
  } catch (e) {
    // 失败回滚
    track.isLiked = !target
    message.error(`收藏失败：${String(e)}`)
  }
}

function playSingle(track: Track) {
  const tracks = (playlist.value?.tracks || []).map(toApiTrack)
  const index = Math.max(tracks.findIndex((t) => t.id === track.id), 0)
  // 播放列表与歌单同步：用当前歌单替换历史队列
  player.playPlaylist(tracks, index)
  router.push({ name: 'Player' })
}

/** 省略号菜单选择处理 */
function onMenuSelect(key: string, track: Track) {
  if (key === 'play') {
    playSingle(track)
  } else if (key === 'play-next') {
    player.playNext(toApiTrack(track))
  } else if (key === 'like') {
    toggleLike(track)
  }
}

function playAll() {
  const tracks = (playlist.value?.tracks || []).map(toApiTrack)
  // 播放列表与歌单同步：用当前歌单替换历史队列
  player.playPlaylist(tracks, 0)
  router.push({ name: 'Player' })
}

function shufflePlay() {
  const tracks = (playlist.value?.tracks || []).map(toApiTrack)
  const shuffled = [...tracks].sort(() => Math.random() - 0.5)
  // 播放列表与歌单同步：用打乱后的歌单替换历史队列
  player.playPlaylist(shuffled, 0)
  player.cyclePlayMode() // 切到随机模式
  router.push({ name: 'Player' })
}

async function loadDetail() {
  const playlistId = String(route.params.id)
  const source = (route.query.source as MusicSource) || 'netease'
  detailSource.value = source
  loading.value = true
  errorMsg.value = ''
  needLogin.value = false
  try {
    const detail = await getPlaylistDetail(source, playlistId)
    playlist.value = {
      id: detail.id,
      name: detail.name,
      description: detail.description || '暂无描述',
      cover: detail.cover_url || fallbackCover(detail.id),
      trackCount: detail.track_count,
      tracks: detail.tracks.map((t) => ({
        id: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album || '未知专辑',
        duration: formatDuration(t.duration || undefined),
        isLiked: false,
        source: t.source,
        coverUrl: t.cover_url,
      })),
    }
  } catch (e) {
    const err = String(e)
    errorMsg.value = err
    playlist.value = null
    // QQ 歌单详情接口要求登录，识别并引导
    needLogin.value = source === 'qq_music' && err.includes('登录')
  } finally {
    loading.value = false
  }
}

onMounted(loadDetail)
</script>

<style scoped>
.state-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 96px 0;
}

/* 歌单头部：封面在左，详情在右 */
.detail-header {
  display: flex;
  align-items: flex-start;
  gap: 32px;
  padding: 8px 4px;
}

.detail-cover {
  width: 240px;
  height: 240px;
  flex-shrink: 0;
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
}

.detail-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding-top: 8px;
}

.detail-title {
  margin: 0 0 14px;
  font-size: 30px;
  font-weight: 700;
  line-height: 1.25;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-meta {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.detail-desc {
  margin: 0 0 24px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--n-text-color-3);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  max-width: 720px;
}

.detail-actions {
  display: flex;
  gap: 12px;
  margin-top: auto;
}

@media (max-width: 640px) {
  .detail-header {
    flex-direction: column;
    align-items: center;
  }
  .detail-info {
    align-items: center;
    text-align: center;
  }
  .detail-title {
    white-space: normal;
  }
}

</style>
