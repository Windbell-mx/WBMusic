<template>
  <div style="padding: 8px">
    <!-- 搜索区 -->
    <n-space vertical size="large" style="margin-bottom: 24px">
      <n-h1 style="margin: 0; font-size: 32px; font-weight: 700">搜索</n-h1>
      <n-space align="center" style="width: 100%">
        <n-input
          v-model:value="keyword"
          size="large"
          clearable
          placeholder="搜索歌曲 / 歌手 / 专辑"
          style="max-width: 480px"
          @keyup.enter="doSearch"
        >
          <template #prefix>
            <n-icon :component="Search" />
          </template>
        </n-input>
        <n-button type="primary" size="large" :loading="loading" @click="doSearch">
          搜索
        </n-button>
        <n-radio-group v-model:value="source" size="large">
          <n-radio-button value="all">全部</n-radio-button>
          <n-radio-button value="qq_music">QQ 音乐</n-radio-button>
          <n-radio-button value="netease">网易云</n-radio-button>
        </n-radio-group>
      </n-space>
    </n-space>

    <!-- 结果区 -->
    <n-empty
      v-if="!searched"
      description="输入关键词开始搜索，选择音乐源可指定平台"
      style="margin-top: 80px"
    />
    <n-empty v-else-if="tracks.length === 0" description="没有找到相关歌曲" style="margin-top: 80px" />
    <n-spin v-else :show="loading">
      <n-list bordered style="border-radius: 12px">
        <n-list-item v-for="track in tracks" :key="`${track.source}-${track.id}`">
          <n-space align="center" justify="space-between" style="width: 100%">
            <n-space align="center" :size="14">
              <!-- 封面 -->
              <n-image
                v-if="track.cover_url"
                :src="track.cover_url"
                width="48"
                height="48"
                object-fit="cover"
                style="border-radius: 8px; flex-shrink: 0"
                :img-props="{ style: 'border-radius: 8px' }"
              />
              <div v-else style="width: 48px; height: 48px; border-radius: 8px; background: rgba(128,128,128,.15); flex-shrink: 0; display:flex; align-items:center; justify-content:center">
                <n-icon :component="MusicalNote" size="22" :depth="3" />
              </div>
              <n-space vertical :size="2">
                <n-text strong style="font-size: 14px">{{ track.title }}</n-text>
                <n-text depth="3" style="font-size: 12px">
                  {{ track.artist }}
                  <template v-if="track.album"> · {{ track.album }}</template>
                  <template v-if="track.duration"> · {{ formatDuration(track.duration) }}</template>
                </n-text>
              </n-space>
            </n-space>
            <n-space align="center" :size="10">
              <n-tag size="small" :type="track.source === 'qq_music' ? 'success' : 'error'" round>
                {{ track.source === 'qq_music' ? 'QQ音乐' : '网易云' }}
              </n-tag>
              <n-button
                type="primary"
                size="small"
                round
                @click="play(track)"
              >
                <template #icon><n-icon :component="Play" /></template>
                播放
              </n-button>
            </n-space>
          </n-space>
        </n-list-item>
      </n-list>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon, useMessage } from 'naive-ui'
import { Search, Play, MusicalNote } from '@vicons/ionicons5'
import { searchMusic, type MusicSource, type Track } from '@/api'
import { usePlayerStore } from '@/stores/player'

const router = useRouter()
const message = useMessage()
const player = usePlayerStore()

const keyword = ref('')
const source = ref<'all' | MusicSource>('all')
const loading = ref(false)
const searched = ref(false)
const tracks = ref<Track[]>([])

async function doSearch() {
  const kw = keyword.value.trim()
  if (!kw) {
    message.warning('请输入搜索关键词')
    return
  }
  loading.value = true
  try {
    const sources: MusicSource[] =
      source.value === 'all' ? ['qq_music', 'netease'] : [source.value]
    const results = await Promise.all(
      sources.map((s) => searchMusic(kw, s, 20).catch(() => null)),
    )
    const merged = results.flatMap((r) => r?.tracks ?? [])
    // 合并后按 (来源,id) 去重
    const seen = new Set<string>()
    tracks.value = merged.filter((t) => {
      const k = `${t.source}-${t.id}`
      if (seen.has(k)) return false
      seen.add(k)
      return true
    })
    searched.value = true
  } catch (e) {
    message.error(`搜索失败: ${e}`)
  } finally {
    loading.value = false
  }
}

function play(track: Track) {
  player.playTrack(track)
  router.push({ name: 'Player' })
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}
</script>
