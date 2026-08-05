<template>
  <Teleport to="body">
  <div class="player-page">
    <!-- 背景装饰光晕 -->
    <div class="bg-glow glow-1"></div>
    <div class="bg-glow glow-2"></div>

    <!-- 顶部导航：歌名 + 歌手信息 -->
    <header class="player-header">
      <n-button text class="header-btn" @click="goBack">
        <n-icon :component="ChevronDown" size="28" />
      </n-button>
      <div class="header-title">
        <div class="header-name">{{ currentTrack?.title }}</div>
        <div class="header-sub">
          <span>{{ currentTrack?.artist }}</span>
          <span class="dot">·</span>
          <span>{{ currentTrack?.album }}</span>
        </div>
      </div>
      <n-button text class="header-btn" @click="showMessage('歌词功能开发中')">
        <n-icon :component="DocumentText" size="22" />
      </n-button>
    </header>

    <!-- 中间主体：左侧唱片 + 右侧歌词 -->
    <main class="player-main">
      <!-- 左侧：唱片 -->
      <section class="disc-section">
        <div class="disc-wrap">
          <div class="disc" :class="{ spinning: isPlaying }">
            <div class="disc-hole"></div>
            <n-image
              v-if="currentTrack?.cover_url"
              :src="currentTrack.cover_url"
              class="disc-cover"
              object-fit="cover"
              :preview-disabled="true"
            />
            <div v-else class="disc-cover disc-placeholder"></div>
          </div>
        </div>
        <div class="track-tags">
          <span class="tag">{{ currentTrack?.album || '未知专辑' }}</span>
          <span class="tag">{{ currentTrack?.artist || '未知歌手' }}</span>
        </div>
      </section>

      <!-- 右侧：歌词 -->
      <section class="lyric-section">
        <div class="lyric-window">
          <div v-if="lyrics.length" class="lyric-list" :style="{ transform: lyricOffset }">
            <div
              v-for="(line, i) in lyrics"
              :key="i"
              class="lyric-line"
              :class="{ active: i === activeLyricIndex }"
            >
              {{ line.text }}
            </div>
          </div>
          <div v-else class="lyric-empty">暂无歌词</div>
        </div>
      </section>
    </main>

    <!-- 底部控制区 -->
    <footer class="player-footer">
      <!-- 进度条 + 红心（同一行） -->
      <div class="progress-row">
        <n-slider
          :value="progress"
          :step="0.1"
          class="progress-slider"
          :format-tooltip="formatTooltip"
          :rail-style="{ backgroundColor: 'rgba(255,255,255,0.15)' }"
          :thumb-style="{ backgroundColor: '#667eea' }"
          @update:value="onProgressChange"
        />
        <n-button text class="ctrl-btn like-btn" @click="toggleLike">
          <n-icon
            :component="isLiked ? Heart : HeartOutline"
            :color="isLiked ? '#ff4d4f' : 'rgba(255,255,255,0.6)'"
            size="20"
          />
        </n-button>
      </div>
      <div class="time-row">
        <span>{{ currentTime }}</span>
        <span>{{ player.totalTime }}</span>
      </div>

      <!-- 控制区：音量 | 播放控制 | 下载/分享（单行平齐） -->
      <div class="control-row">
        <div class="volume-row">
          <n-icon :component="VolumeLow" class="vol-icon" size="18" />
          <n-slider
            :value="volume"
            :step="0.01"
            class="volume-slider"
            :rail-style="{ backgroundColor: 'rgba(255,255,255,0.15)' }"
            :thumb-style="{ backgroundColor: '#667eea' }"
            @update:value="onVolumeChange"
          />
          <n-icon :component="VolumeHigh" class="vol-icon" size="18" />
        </div>

        <div class="controls">
          <n-tooltip trigger="hover" placement="top">
            <template #trigger>
              <n-button text class="mode-btn" @click="cyclePlayMode">
                <n-icon :component="playMode === 'shuffle' ? Shuffle : Repeat" size="20" />
                <span v-if="playMode === 'one'" class="repeat-one">1</span>
              </n-button>
            </template>
            <div class="mode-tip">
              <div>{{ modeTitle }}</div>
              <div class="mode-tip-next">点击切换：{{ nextModeTitle }}</div>
            </div>
          </n-tooltip>

          <n-button text class="ctrl-btn" @click="prevTrack" title="上一首">
            <n-icon :component="PlaySkipBack" size="26" />
          </n-button>

          <n-button circle size="large" type="primary" class="play-btn" @click="togglePlay">
            <template #icon><n-icon :component="isPlaying ? Pause : Play" size="26" /></template>
          </n-button>

          <n-button text class="ctrl-btn" @click="nextTrack" title="下一首">
            <n-icon :component="PlaySkipForward" size="26" />
          </n-button>

          <n-button text class="ctrl-btn" @click="showPlaylistPanel = true">
            <n-icon :component="List" size="20" />
          </n-button>
        </div>

        <div class="action-row">
          <n-button text class="action-btn" @click="showMessage('下载功能开发中')">
            <n-icon :component="Download" size="18" />
            下载
          </n-button>
          <n-button text class="action-btn" @click="showMessage('分享功能开发中')">
            <n-icon :component="ShareSocial" size="18" />
            分享
          </n-button>
        </div>
      </div>
    </footer>

    <!-- 播放列表面板 -->
    <PlaylistPanel v-model:visible="showPlaylistPanel" />
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onBeforeUnmount } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useMessage } from 'naive-ui'
import {
  Play,
  Pause,
  PlaySkipBack,
  PlaySkipForward,
  Shuffle,
  Repeat,
  Heart,
  HeartOutline,
  VolumeLow,
  VolumeHigh,
  ChevronDown,
  DocumentText,
  List,
  Download,
  ShareSocial,
} from '@vicons/ionicons5'
import { getLyrics, getPlaylistDetail, type MusicSource } from '@/api'
import { usePlayerStore } from '@/stores/player'
import PlaylistPanel from '@/components/Player/PlaylistPanel.vue'

const router = useRouter()
const route = useRoute()
const message = useMessage()
const player = usePlayerStore()

// 播放列表面板开关
const showPlaylistPanel = ref(false)

// 播放状态全部来自共享 store（与底部 PlayerBar 联动）
const isPlaying = computed(() => player.isPlaying)
const progress = computed(() => player.progress)
const volume = computed(() => player.volume)
const isLiked = computed(() => player.isLiked)
const playMode = computed(() => player.playMode)
const currentTrack = computed(() => player.currentTrack)
const currentTime = computed(() => player.currentTime)
const modeTitle = computed(() => player.modeTitle)
const nextModeTitle = computed(() => player.nextModeTitle)

// 总时长（秒），用于进度换算
let totalSeconds = 269

// 歌词（时间单位为秒），无真实歌词时为空
const lyrics = ref<{ time: number; text: string }[]>([])

async function loadLyrics() {
  if (!currentTrack.value) return
  try {
    const text = await getLyrics(currentTrack.value.source, currentTrack.value.id)
    if (text) {
      // 解析 LRC 格式歌词
      const lines = text.split('\n')
        .map((line) => {
          const match = line.match(/\[(\d+):(\d+)(?:\.(\d+))?\](.*)/)
          if (match) {
            const time = Number(match[1]) * 60 + Number(match[2]) + Number(match[3] || 0) / 100
            return { time, text: match[4].trim() }
          }
          return null
        })
        .filter((l): l is { time: number; text: string } => l !== null && l.text.length > 0)
      if (lines.length > 0) {
        lyrics.value = lines
        if (lines.length > 0) totalSeconds = Math.max(totalSeconds, lines[lines.length - 1].time + 10)
      }
    }
  } catch {
    /* 保留默认歌词 */
  }
}

// 当前高亮歌词行（由进度实时计算）
const activeLyricIndex = computed(() => {
  const currentSeconds = (progress.value / 100) * totalSeconds
  let index = 0
  for (let i = 0; i < lyrics.value.length; i++) {
    if (currentSeconds >= lyrics.value[i].time) {
      index = i
    } else {
      break
    }
  }
  return index
})

// 歌词列表位移（让高亮行保持在窗口中央，行高 44px，窗口中心偏移 110px）
const lyricOffset = computed(() => {
  return `translateY(${110 - activeLyricIndex.value * 44}px)`
})

function goBack() {
  router.back()
}

function togglePlay() {
  player.togglePlay()
}

function prevTrack() {
  player.prevTrack()
}

function nextTrack() {
  player.nextTrack()
}

function toggleLike() {
  player.toggleLike()
}

function cyclePlayMode() {
  player.cyclePlayMode()
}

function showMessage(content: string) {
  message.info(content, { duration: 2000 })
}

function formatTooltip(value: number) {
  const secs = Math.floor((value / 100) * totalSeconds)
  const minutes = Math.floor(secs / 60)
  const seconds = secs % 60
  return minutes + ':' + seconds.toString().padStart(2, '0')
}

function onProgressChange(value: number) {
  player.seek(value)
}

function onVolumeChange(value: number) {
  player.setVolume(value)
}

// 曲目切换时重新加载歌词
watch(
  () => player.currentTrack?.id,
  () => {
    loadLyrics()
  },
)

onMounted(() => {
  const playlistId = route.query.playlistId as string | undefined
  const source = (route.query.source as MusicSource) || 'netease'

  if (playlistId) {
    // 从歌单进入：加载歌单并播放（替换历史队列，不叠加）
    getPlaylistDetail(source, playlistId)
      .then((detail) => {
        const tracks = detail.tracks
        if (!tracks.length) {
          message.warning('歌单中没有可播放的歌曲')
          return
        }
        player.playPlaylist(tracks, 0)
      })
      .catch((e) => {
        message.error(`加载歌单失败: ${e}`)
      })
  } else if (!player.currentTrack) {
    // 无正在播放的曲目：提示用户去歌单/搜索选择，不自动载入
    message.info('暂无播放中的歌曲，去歌单或搜索中选择一首吧')
  }
  loadLyrics()
})

onBeforeUnmount(() => {
  // 不暂停播放：返回时底部 PlayerBar 继续播放
})
</script>

<style scoped>
.player-page {
  position: fixed;
  inset: 0;
  z-index: 1000;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 55%, #0f3460 100%);
  color: #fff;
}

/* 背景光晕 */
.bg-glow {
  position: absolute;
  border-radius: 50%;
  filter: blur(120px);
  opacity: 0.35;
  pointer-events: none;
}

.glow-1 {
  width: 480px;
  height: 480px;
  background: radial-gradient(circle, rgba(102, 126, 234, 0.9), transparent 70%);
  top: -140px;
  right: -100px;
}

.glow-2 {
  width: 420px;
  height: 420px;
  background: radial-gradient(circle, rgba(118, 75, 162, 0.8), transparent 70%);
  bottom: -120px;
  left: -80px;
}

/* 顶部导航 */
.player-header {
  position: relative;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
}

.header-btn {
  color: rgba(255, 255, 255, 0.8);
  transition: color 0.2s, transform 0.2s;
}

.header-btn:hover {
  color: #fff;
  transform: scale(1.1);
}

.header-title {
  text-align: center;
  min-width: 0;
}

.header-name {
  font-size: 18px;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.header-sub {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  margin-top: 3px;
}

.header-sub .dot {
  opacity: 0.5;
}

/* 主体：唱片 + 歌词 */
.player-main {
  position: relative;
  z-index: 10;
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: center;
  gap: 40px;
  padding: 0 6vw;
  min-height: 0;
  overflow: hidden;
}

/* 左侧：唱片 */
.disc-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.disc-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
}

.disc {
  position: relative;
  width: min(300px, 32vh);
  height: min(300px, 32vh);
  border-radius: 50%;
  background: radial-gradient(circle at center, #2a2a3a 0%, #16161f 60%, #0c0c14 100%);
  box-shadow:
    0 30px 80px rgba(0, 0, 0, 0.6),
    inset 0 0 40px rgba(0, 0, 0, 0.5),
    0 0 0 10px rgba(255, 255, 255, 0.03),
    0 0 0 14px rgba(255, 255, 255, 0.02);
}

.disc-cover {
  position: absolute;
  top: 18%;
  left: 18%;
  width: 64%;
  height: 64%;
  border-radius: 50%;
  box-shadow: 0 0 30px rgba(0, 0, 0, 0.6);
}

.disc-placeholder {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04));
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.disc-hole {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 12%;
  height: 12%;
  border-radius: 50%;
  background: radial-gradient(circle, #667eea 0%, #764ba2 100%);
  box-shadow: 0 0 12px rgba(102, 126, 234, 0.7);
  z-index: 2;
}

.disc.spinning {
  animation: disc-spin 16s linear infinite;
}

@keyframes disc-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.track-tags {
  display: flex;
  gap: 10px;
}

.tag {
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.85);
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(8px);
}

/* 右侧：歌词 */
.lyric-section {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 0;
}

.lyric-window {
  position: relative;
  width: 100%;
  max-width: 440px;
  height: min(340px, 40vh);
  overflow: hidden;
  -webkit-mask-image: linear-gradient(
    to bottom,
    transparent 0%,
    rgba(0, 0, 0, 0.9) 22%,
    #000 42%,
    #000 58%,
    rgba(0, 0, 0, 0.9) 78%,
    transparent 100%
  );
  mask-image: linear-gradient(
    to bottom,
    transparent 0%,
    rgba(0, 0, 0, 0.9) 22%,
    #000 42%,
    #000 58%,
    rgba(0, 0, 0, 0.9) 78%,
    transparent 100%
  );
}

.lyric-list {
  display: flex;
  flex-direction: column;
  padding-top: 110px;
  transition: transform 0.5s ease;
}

.lyric-line {
  height: 44px;
  line-height: 44px;
  text-align: center;
  font-size: 15px;
  color: rgba(255, 255, 255, 0.35);
  white-space: nowrap;
  transition: color 0.4s ease, font-size 0.4s ease;
}

.lyric-line.active {
  color: #fff;
  font-size: 19px;
  font-weight: 600;
  text-shadow: 0 0 20px rgba(102, 126, 234, 0.8);
}

.lyric-empty {
  text-align: center;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.35);
}

/* 底部控制区 */
.player-footer {
  position: relative;
  z-index: 10;
  padding: 12px 6vw 14px;
  background: rgba(10, 12, 28, 0.55);
  backdrop-filter: blur(24px);
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

/* 进度条 + 红心（同一行） */
.progress-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.progress-slider {
  flex: 1;
  min-width: 0;
}

.like-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.time-row {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.45);
  font-variant-numeric: tabular-nums;
  margin-top: -2px;
}

.controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 24px;
}

.mode-btn {
  color: #667eea;
  transition: color 0.2s, transform 0.2s;
  position: relative;
}

.mode-btn:hover {
  color: #8b9cf5;
  transform: scale(1.1);
}

.mode-tip {
  text-align: center;
}

.mode-tip-next {
  font-size: 11px;
  opacity: 0.6;
  margin-top: 2px;
}

.repeat-one {
  position: absolute;
  top: -4px;
  right: -8px;
  font-size: 10px;
  font-weight: 700;
  color: #667eea;
  background: rgba(22, 33, 62, 0.9);
  border-radius: 50%;
  padding: 0 4px;
  line-height: 1.4;
}

.ctrl-btn {
  color: rgba(255, 255, 255, 0.7);
  transition: color 0.2s, transform 0.2s;
}

.ctrl-btn:hover {
  color: #fff;
  transform: scale(1.1);
}

.play-btn {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%) !important;
  border: none !important;
  box-shadow: 0 6px 24px rgba(102, 126, 234, 0.5);
  transition: transform 0.2s, box-shadow 0.2s;
}

.play-btn:hover {
  transform: scale(1.08);
  box-shadow: 0 10px 32px rgba(102, 126, 234, 0.65);
}

/* 控制区：音量 | 播放控制 | 下载/分享 单行平齐 */
.control-row {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  margin-top: 4px;
}

.volume-row {
  display: flex;
  align-items: center;
  gap: 10px;
  justify-self: start;
}

.vol-icon {
  color: rgba(255, 255, 255, 0.5);
}

.volume-slider {
  width: 120px;
}

.action-row {
  display: flex;
  align-items: center;
  gap: 20px;
  justify-self: end;
}

.action-btn {
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
  transition: color 0.2s;
}

.action-btn:hover {
  color: #fff;
}

/* 响应式（播放页为全屏覆盖层，可用宽度 = 视口宽度） */
@media (max-width: 900px) {
  .player-page {
    overflow-y: auto;
  }

  .player-main {
    grid-template-columns: 1fr;
    gap: 16px;
    padding: 0 24px 24px;
    align-items: center;
  }

  .disc {
    width: min(220px, 28vh);
    height: min(220px, 28vh);
  }

  .lyric-window {
    height: 240px;
  }

  .header-name {
    font-size: 16px;
  }

  .control-row {
    grid-template-columns: 1fr;
    justify-items: center;
    gap: 12px;
  }
}
</style>
