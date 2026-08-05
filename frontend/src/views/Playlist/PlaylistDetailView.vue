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
        <n-empty description="歌单加载失败">
          <template #extra>
            <n-text depth="3">{{ errorMsg }}</n-text>
          </template>
        </n-empty>
      </div>

      <template v-else>
        <n-grid cols="s:1 m:2 xl:3" x-gap="24" y-gap="24" style="margin-bottom: 24px">
          <n-gi>
            <n-image
              :src="playlist.cover"
              :alt="playlist.name"
              style="width: 240px; height: 240px; border-radius: 8px; box-shadow: var(--n-box-shadow)"
              object-fit="cover"
            />
          </n-gi>
          <n-gi>
            <n-space vertical>
              <n-h1 style="margin: 0">{{ playlist.name }}</n-h1>
              <n-text depth="3">{{ playlist.description }}</n-text>
              <n-space>
                <n-tag>{{ playlist.trackCount }} 首歌曲</n-tag>
                <n-tag>{{ playlist.tracks[0]?.source === 'netease' ? '网易云' : 'QQ 音乐' }}</n-tag>
              </n-space>
              <n-button type="primary" size="large" @click="playAll">
                <template #icon>
                  <n-icon :component="Play" />
                </template>
                播放
              </n-button>
            </n-space>
          </n-gi>
        </n-grid>

        <n-divider />

        <n-data-table
          :columns="columns"
          :data="playlist.tracks || []"
          :pagination="{ pageSize: 20 }"
          striped
        />
      </template>
    </n-layout-content>
  </n-layout>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NIcon, NButton, NSpin, NEmpty, NSpace } from 'naive-ui'
import {
  Play,
  Shuffle,
  ArrowBack,
  Mic,
  Time,
  Star,
  Ellipse,
} from '@vicons/ionicons5'
import type { DataTableColumns } from 'naive-ui'
import { usePlayerStore } from '@/stores/player'
import {
  getPlaylistDetail,
  type Track as ApiTrack,
  type MusicSource,
} from '@/api'

const router = useRouter()
const route = useRoute()
const player = usePlayerStore()

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

const columns: DataTableColumns<Track> = [
  {
    title: '#',
    key: 'index',
    width: 60,
    render: (_row, index) => index + 1,
  },
  {
    title: '歌曲',
    key: 'title',
    width: 300,
    render: (row) => {
      return h(
        NSpace,
        { align: 'center' },
        {
          default: () => [
            h(NIcon, { component: Mic }),
            h(
              'a',
              {
                style: 'cursor: pointer; color: inherit; text-decoration: none',
                onClick: () => playSingle(row),
              },
              row.title,
            ),
          ],
        },
      )
    },
  },
  {
    title: '歌手',
    key: 'artist',
    width: 200,
  },
  {
    title: '专辑',
    key: 'album',
    width: 200,
  },
  {
    title: '时长',
    key: 'duration',
    width: 80,
    render: (row) => {
      return h(
        NSpace,
        { align: 'center' },
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
    width: 100,
    render: (row) => {
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
              NButton,
              { text: true },
              { default: () => h(NIcon, { component: Ellipse }) }
            ),
          ],
        },
      )
    },
  },
]

function toggleLike(track: Track) {
  track.isLiked = !track.isLiked
}

function playSingle(track: Track) {
  const tracks = (playlist.value?.tracks || []).map(toApiTrack)
  const index = Math.max(tracks.findIndex((t) => t.id === track.id), 0)
  // 播放列表与歌单同步：用当前歌单替换历史队列
  player.playPlaylist(tracks, index)
  router.push({ name: 'Player' })
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

onMounted(async () => {
  const playlistId = String(route.params.id)
  const source = (route.query.source as MusicSource) || 'netease'
  loading.value = true
  errorMsg.value = ''
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
    errorMsg.value = String(e)
    playlist.value = null
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.state-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 96px 0;
}
</style>
