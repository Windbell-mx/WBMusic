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
      <n-button text class="header-btn" @click="appStore.showLyrics = !appStore.showLyrics">
        <n-icon
          :component="DocumentText"
          size="22"
          :color="appStore.showLyrics ? appStore.themeColor : 'rgba(255,255,255,0.6)'"
        />
      </n-button>
    </header>

    <!-- 中间主体：左侧唱片 + 右侧歌词 -->
    <main class="player-main" :class="{ 'no-lyrics': !appStore.showLyrics }">
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
            <div
              v-else
              class="disc-cover disc-placeholder"
              :style="{ backgroundColor: appStore.themeColor }"
            >
              <n-icon :component="MusicalNotes" class="disc-placeholder-icon" />
            </div>
          </div>
        </div>
        <div class="track-tags">
          <span class="next-badge">下一首</span>
          <div class="next-wrap">
            <div class="next-scroll scrolling">
              <div class="next-scroll-track">
                <span class="next-group">
                  <span class="next-title">{{ nextTrackInfo?.title || '暂无' }}</span>
                  <span v-if="nextTrackInfo?.artist" class="next-artist"> - {{ nextTrackInfo.artist }}</span>
                </span>
                <span class="next-group" aria-hidden="true">
                  <span class="next-title">{{ nextTrackInfo?.title || '暂无' }}</span>
                  <span v-if="nextTrackInfo?.artist" class="next-artist"> - {{ nextTrackInfo.artist }}</span>
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- 右侧：歌词 -->
      <section v-if="appStore.showLyrics" class="lyric-section">
        <div class="lyric-window" ref="lyricWindowEl" @wheel.prevent="onLyricWheel">
          <div v-if="lyrics.length" class="lyric-list" :class="{ manual: isManualScroll }" :style="{ transform: lyricOffset }">
            <div
              v-for="(line, i) in lyrics"
              :key="i"
              class="lyric-line"
              :class="{ active: i === activeLyricIndex }"
              @click="onLyricClick(line)"
            >
              {{ line.text }}
            </div>
          </div>
          <div v-else class="lyric-empty">暂无歌词</div>

          <!-- 手动浏览时：回到当前播放位置 -->
          <transition name="fade">
            <button v-if="isManualScroll" class="lyric-back-btn" @click="backToPlaying">
              <n-icon :component="MusicalNotes" size="14" />
              回到当前播放
            </button>
          </transition>
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
          :thumb-style="{ backgroundColor: appStore.themeColor }"
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
            :thumb-style="{ backgroundColor: appStore.themeColor }"
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
  MusicalNotes,
} from '@vicons/ionicons5'
import { getLyrics, getPlaylistDetail, type MusicSource } from '@/api'
import { usePlayerStore } from '@/stores/player'
import { useAppStore } from '@/stores/app'
import PlaylistPanel from '@/components/Player/PlaylistPanel.vue'

const router = useRouter()
const route = useRoute()
const message = useMessage()
const player = usePlayerStore()
const appStore = useAppStore()

// 播放列表面板开关
const showPlaylistPanel = ref(false)

// 播放状态全部来自共享 store（与底部 PlayerBar 联动）
const isPlaying = computed(() => player.isPlaying)
const progress = computed(() => player.progress)
const volume = computed(() => player.volume)
const isLiked = computed(() => player.isLiked)
const playMode = computed(() => player.playMode)
const currentTrack = computed(() => player.currentTrack)
const nextTrackInfo = computed(() => player.nextTrackInfo)
const currentTime = computed(() => player.currentTime)
const modeTitle = computed(() => player.modeTitle)
const nextModeTitle = computed(() => player.nextModeTitle)

// 歌词（时间单位为秒），无真实歌词时为空
const lyrics = ref<{ time: number; text: string }[]>([])

// 歌词窗口元素与高度（响应式，用于高亮行居中计算）
const lyricWindowEl = ref<HTMLElement | null>(null)
const lyricWindowH = ref(340)

function updateLyricWindowH() {
  lyricWindowH.value = lyricWindowEl.value?.offsetHeight ?? 340
}

// 歌词请求序号：防止快速切歌时旧请求晚返回覆盖新歌歌词
let lyricReqSeq = 0

async function loadLyrics() {
  const seq = ++lyricReqSeq
  // 切换到新曲目先清空歌词，避免上一首的歌词残留滚动
  lyrics.value = []
  // 重置手动浏览状态
  clearTimeout(manualScrollTimer)
  isManualScroll.value = false
  manualOffset.value = 0
  if (!currentTrack.value) return
  try {
    const text = await getLyrics(currentTrack.value.source, currentTrack.value.id)
    // 请求期间已切到其他歌曲，丢弃过期结果
    if (seq !== lyricReqSeq) return
    if (text) {
      // 解析 LRC 格式歌词
      const lines = text.split('\n')
        .map((line) => {
          const match = line.match(/\[(\d+):(\d+)(?:\.(\d+))?\](.*)/)
          if (match) {
            // 兼容两种时间戳精度：标准 LRC 的 [mm:ss.xx]（百分秒）与 [mm:ss.xxx]（毫秒）
            const fracMs = match[3] ? Number(match[3].padEnd(3, '0')) / 1000 : 0
            const time = Number(match[1]) * 60 + Number(match[2]) + fracMs
            return { time, text: match[4].trim() }
          }
          return null
        })
        .filter((l): l is { time: number; text: string } => l !== null && l.text.length > 0)
      if (lines.length > 0) {
        lyrics.value = lines
      }
    }
  } catch {
    /* 请求失败保持空（显示暂无歌词） */
  }
}

// 当前高亮歌词行（直接用音频真实当前秒数，与声音精确同步）
const activeLyricIndex = computed(() => {
  const currentSeconds = player.currentSec
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

// 歌词列表位移：让高亮行始终保持在歌词窗口中央
// 窗口高 H，行高 44px → 高亮行中心应位于 H/2，列表位移 = H/2 - 行高/2 - 当前行号*44
const lyricOffset = computed(() => {
  if (isManualScroll.value) {
    return `translateY(${manualOffset.value}px)`
  }
  const offset = lyricWindowH.value / 2 - 22
  return `translateY(${offset - activeLyricIndex.value * 44}px)`
})

// ---- 歌词手动浏览：滚轮滚动 + 点击跳转 ----
// 手动滚动偏移（px），进入手动模式后不再跟随播放自动滚动
const manualOffset = ref(0)
// 是否处于手动浏览模式（滚轮滚动后进入；点击歌词跳转或"回到当前"退出）
const isManualScroll = ref(false)

// 歌词行高（与 CSS .lyric-line 保持一致）
const LYRIC_LINE_H = 44

// 停止滚动后自动跳回播放位置的延迟（毫秒）
const MANUAL_SCROLL_IDLE_MS = 3000
let manualScrollTimer: number | undefined

function onLyricWheel(e: WheelEvent) {
  if (lyrics.value.length === 0) return
  // 首次滚动：以当前高亮行位置为起点进入手动模式
  if (!isManualScroll.value) {
    manualOffset.value = lyricWindowH.value / 2 - LYRIC_LINE_H / 2 - activeLyricIndex.value * LYRIC_LINE_H
    isManualScroll.value = true
  }
  // 向下滚动（deltaY>0）→ 列表上移，浏览后续歌词
  manualOffset.value -= e.deltaY
  // 边界限制：第一行居中（最大值）与最后一行居中（最小值）之间
  const maxOffset = lyricWindowH.value / 2 - LYRIC_LINE_H / 2
  const minOffset = maxOffset - (lyrics.value.length - 1) * LYRIC_LINE_H
  manualOffset.value = Math.min(maxOffset, Math.max(minOffset, manualOffset.value))

  // 重置闲置计时：停止滚动一段时间后自动跳回当前播放位置
  clearTimeout(manualScrollTimer)
  manualScrollTimer = window.setTimeout(backToPlaying, MANUAL_SCROLL_IDLE_MS)
}

/** 点击歌词行：跳转到该行时间点播放，并恢复自动跟随 */
function onLyricClick(line: { time: number; text: string }) {
  player.seekTo(line.time)
  clearTimeout(manualScrollTimer)
  isManualScroll.value = false
}

/** 退出手动浏览，恢复跟随当前播放位置 */
function backToPlaying() {
  clearTimeout(manualScrollTimer)
  isManualScroll.value = false
}

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
  const secs = Math.floor((value / 100) * (player.audioDuration || 0))
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

// 暂停/停止播放时自动退出手动浏览，歌词跳回当前播放位置
watch(
  () => player.isPlaying,
  (playing) => {
    if (!playing) {
      isManualScroll.value = false
    }
  },
)

// 歌词区被 v-if 隐藏后再显示时，窗口高度可能已变化（窗口缩放等），重新测量保证高亮行居中
watch(
  () => appStore.showLyrics,
  (visible) => {
    if (visible) {
      requestAnimationFrame(updateLyricWindowH)
    }
  },
)

onMounted(() => {
  // 测量歌词窗口实际高度（受窗口大小影响），用于高亮行居中
  requestAnimationFrame(updateLyricWindowH)
  window.addEventListener('resize', updateLyricWindowH)
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
  window.removeEventListener('resize', updateLyricWindowH)
  clearTimeout(manualScrollTimer)
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
  /* 顶部为透明标题栏（36px）留出空间，背景渐变铺满整个窗口 */
  padding-top: 36px;
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
  background: radial-gradient(circle, color-mix(in srgb, var(--accent) 90%, transparent), transparent 70%);
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

.header-btn:focus {
  color: rgba(255, 255, 255, 0.8) !important;
}

.header-btn:hover,
.header-btn:focus:hover {
  color: #fff !important;
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

/* 隐藏歌词时：唱片居中单列 */
.player-main.no-lyrics {
  grid-template-columns: 1fr;
  justify-items: center;
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
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(255, 255, 255, 0.12);
}

.disc-placeholder-icon {
  font-size: 40%;
  color: rgba(255, 255, 255, 0.9);
  filter: drop-shadow(0 4px 10px rgba(0, 0, 0, 0.35));
}

.disc-hole {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 12%;
  height: 12%;
  border-radius: 50%;
  /* 玻璃透镜基底：中心略亮的深色，边缘过渡到唱片 */
  background: radial-gradient(circle at 35% 30%, rgba(255, 255, 255, 0.5) 0%, rgba(255, 255, 255, 0.08) 8%, transparent 22%),
    radial-gradient(circle, #14141d 0%, #0a0a11 45%, #23232e 100%);
  box-shadow:
    inset 0 2px 8px rgba(0, 0, 0, 0.95),
    inset -2px -4px 10px rgba(255, 255, 255, 0.08),
    0 0 16px rgba(255, 255, 255, 0.12),
    0 0 0 2px rgba(255, 255, 255, 0.05);
  z-index: 2;
  overflow: hidden;
}

/* 折射光线（随唱片整体旋转，不再独立自转，保证与封面同速） */
.disc-hole::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: conic-gradient(
    from 0deg,
    transparent 0deg,
    rgba(255, 255, 255, 0.3) 16deg,
    rgba(255, 255, 255, 0.06) 42deg,
    transparent 85deg,
    transparent 170deg,
    rgba(102, 126, 234, 0.25) 200deg,
    rgba(255, 255, 255, 0.08) 225deg,
    transparent 265deg,
    transparent 360deg
  );
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
  align-items: center;
  gap: 10px;
  width: min(320px, 60vw);
  min-height: 26px;
}

.next-badge {
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 2px;
  color: rgba(255, 255, 255, 0.4);
}

.next-wrap {
  min-width: 0;
  flex: 1;
  overflow: hidden;
}

.next-scroll {
  overflow: hidden;
  white-space: nowrap;
  min-width: 0;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}

.next-scroll-track {
  display: inline-flex;
  white-space: nowrap;
  will-change: transform;
  align-items: baseline;
}

.next-scroll.scrolling .next-scroll-track {
  animation: next-marquee 8s linear infinite;
}

.next-group {
  display: inline-flex;
  white-space: nowrap;
  align-items: baseline;
  padding-right: 48px;
}

.next-title {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.4;
  color: rgba(255, 255, 255, 0.95);
  text-shadow: 0 0 8px rgba(255, 255, 255, 0.25);
  letter-spacing: 0.5px;
}

.next-artist {
  font-size: 12px;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.6);
  text-shadow: 0 0 6px rgba(255, 255, 255, 0.12);
  margin-left: 6px;
}

@keyframes next-marquee {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(-50%);
  }
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
  /* 0.3s：比 0.5s 更跟手，滚动及时跟上音频进度，减少视觉不同步 */
  transition: transform 0.3s ease;
}

/* 手动浏览模式：滚轮直接控制位移，无需平滑过渡动画 */
.lyric-list.manual {
  transition: none;
}

.lyric-line {
  height: 44px;
  line-height: 44px;
  text-align: center;
  font-size: 15px;
  color: rgba(255, 255, 255, 0.35);
  white-space: nowrap;
  transition: color 0.4s ease, font-size 0.4s ease;
  cursor: pointer;
  padding: 0 8px;
  border-radius: 6px;
}

.lyric-line:hover {
  color: rgba(255, 255, 255, 0.75);
  background: rgba(255, 255, 255, 0.06);
}

.lyric-line.active {
  color: #fff;
  font-size: 19px;
  font-weight: 600;
  text-shadow: 0 0 20px color-mix(in srgb, var(--accent) 80%, transparent);
}

.lyric-line.active:hover {
  background: color-mix(in srgb, var(--accent) 12%, transparent);
}

/* 手动浏览时"回到当前播放"按钮 */
.lyric-back-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 10px;
  border: none;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 25%, transparent);
  color: #fff;
  font-size: 12px;
  cursor: pointer;
  backdrop-filter: blur(6px);
  transition: background 0.2s;
  z-index: 5;
}

.lyric-back-btn:hover {
  background: color-mix(in srgb, var(--accent) 45%, transparent);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.lyric-empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  font-size: 16px;
  letter-spacing: 1px;
  color: rgba(255, 255, 255, 0.6);
  user-select: none;
}

/* 底部控制区 */
.player-footer {
  position: relative;
  z-index: 10;
  padding: 12px 6vw 14px;
  /* 半透明背景代替毛玻璃：blur(24px) 全宽常驻在 WebView2 上
     滚动歌词/切歌时每帧重采样背景，会卡。半透明色视觉接近且零开销 */
  background: rgba(10, 12, 28, 0.72);
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
  color: var(--accent-light);
  transition: color 0.2s, transform 0.2s;
  position: relative;
}

.mode-btn:focus {
  color: var(--accent-light) !important;
}

.mode-btn:hover,
.mode-btn:focus:hover {
  color: #8b9cf5 !important;
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
  color: var(--accent-light);
  background: rgba(22, 33, 62, 0.9);
  border-radius: 50%;
  padding: 0 4px;
  line-height: 1.4;
}

.ctrl-btn {
  color: rgba(255, 255, 255, 0.7);
  transition: color 0.2s, transform 0.2s;
}

/* 点击后按钮会保留键盘焦点，naive-ui 默认在 :focus 套用主题色（表现为"按下去后颜色不恢复"）。
   用 !important 压过 naive-ui 的 :focus 规则恢复常态色；悬停规则放其后，悬停时仍正常高亮 */
.ctrl-btn:focus {
  color: rgba(255, 255, 255, 0.7) !important;
}

.ctrl-btn:hover,
.ctrl-btn:focus:hover {
  color: #fff !important;
  transform: scale(1.1);
}

.play-btn {
  /* 透明毛玻璃：无底色 + 模糊 + 主题色描边（播放页深色背景上图标保持白色） */
  background: rgba(255, 255, 255, 0.08) !important;
  border: 1px solid color-mix(in srgb, var(--accent-light) 45%, transparent) !important;
  backdrop-filter: blur(16px) saturate(1.6);
  -webkit-backdrop-filter: blur(16px) saturate(1.6);
  color: #fff !important;
  box-shadow: 0 6px 24px color-mix(in srgb, var(--accent) 30%, transparent);
  transition: transform 0.2s, box-shadow 0.2s;
}

.play-btn:hover {
  transform: scale(1.08);
  box-shadow: 0 10px 32px color-mix(in srgb, var(--accent) 45%, transparent);
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

.action-btn:focus {
  color: rgba(255, 255, 255, 0.5) !important;
}

.action-btn:hover,
.action-btn:focus:hover {
  color: #fff !important;
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
