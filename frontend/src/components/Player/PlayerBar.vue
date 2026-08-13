<template>
  <n-layout position="absolute" bottom class="player-bar" style="top: auto; height: 84px">
    <!-- 顶部渐变光晕分隔线 -->
    <div class="bar-glow"></div>

    <n-layout-content style="height: 100%; background: transparent">
      <div class="player-grid">
        <!-- 左侧：歌曲信息（点击进入播放详情页） -->
        <div class="player-track" @click="goToPlayer" title="打开播放页">
          <n-image
            :src="currentTrack?.cover_url || 'https://picsum.photos/seed/default/60/60'"
            class="track-cover"
            object-fit="cover"
            :preview-disabled="true"
          />
          <div class="track-info">
            <n-text class="track-title">{{ currentTrack?.title || '未在播放' }}</n-text>
            <n-text class="track-artist">{{ currentTrack?.artist || '选择一首歌曲开始播放' }}</n-text>
          </div>
          <n-button text class="like-btn" :class="{ liked: isLiked }" @click.stop="toggleLike">
            <n-icon :component="isLiked ? Heart : HeartOutline" :color="isLiked ? '#ff4d4f' : 'var(--n-text-color-3)'" size="18" />
          </n-button>
        </div>

        <!-- 中间：控制按钮 + 进度条 -->
        <div class="player-center">
          <div class="player-controls">
            <n-tooltip trigger="hover" placement="top">
              <template #trigger>
                <n-button text class="ctrl-btn mode-btn" @click="cyclePlayMode">
                  <n-icon :component="playMode === 'shuffle' ? Shuffle : Repeat" size="18" />
                  <span v-if="playMode === 'one'" class="repeat-one">1</span>
                </n-button>
              </template>
              <div class="mode-tip">
                <div class="mode-tip-name">{{ modeTitle }}</div>
                <div class="mode-tip-next">点击切换：{{ nextModeTitle }}</div>
              </div>
            </n-tooltip>
            <n-button text class="ctrl-btn" @click="prevTrack" title="上一首">
              <n-icon :component="PlaySkipBack" size="22" />
            </n-button>
            <n-button circle type="primary" class="play-btn" @click="togglePlay">
              <template #icon><n-icon :component="isPlaying ? Pause : Play" size="20" /></template>
            </n-button>
            <n-button text class="ctrl-btn" @click="nextTrack" title="下一首">
              <n-icon :component="PlaySkipForward" size="22" />
            </n-button>
          </div>

          <div class="player-progress">
            <n-text class="time-label">{{ currentTime }}</n-text>
            <n-slider
              :value="progress"
              :step="0.1"
              class="progress-slider"
              :rail-style="{ backgroundColor: 'var(--n-border-color)' }"
              @update:value="onProgressChange"
            />
            <n-text class="time-label">{{ player.totalTime }}</n-text>
          </div>
        </div>

        <!-- 右侧：音量 + 操作 -->
        <div class="player-right">
          <div class="player-volume">
            <n-icon :component="VolumeLow" class="vol-icon" size="16" />
            <n-slider :value="volume" :step="0.01" class="volume-slider" :rail-style="{ backgroundColor: 'var(--n-border-color)' }" @update:value="onVolumeChange" />
            <n-icon :component="VolumeHigh" class="vol-icon" size="16" />
          </div>
          <n-button text class="ctrl-btn" @click="showPlaylistPanel = true" title="播放列表">
            <n-icon :component="List" size="18" />
          </n-button>
          <n-button text class="ctrl-btn" @click="goToPlayer" title="打开播放页">
            <n-icon :component="Headset" size="18" />
          </n-button>
        </div>
      </div>
    </n-layout-content>
  </n-layout>

  <!-- 播放列表面板 -->
  <Teleport to="body">
    <PlaylistPanel v-model:visible="showPlaylistPanel" />
  </Teleport>
</template>

<style scoped>
.player-bar {
  /* 注意：必须用 fixed 而非 absolute —— DefaultLayout 为让详情页 sticky
     头部生效，已把内层主内容区 overflow 改为 visible（滚动发生在最外层
     容器），若用 absolute 会相对整个内容高度定位，滚动时播放器跟着滚走。 */
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 100;
  /* 半透明纯色背景代替毛玻璃：fixed 元素若用 backdrop-filter，
     滚动/路由切换时 WebView2 每帧重采样全宽背景，交互极卡。
     半透明色视觉接近毛玻璃且零重采样开销 */
  background: rgba(255, 255, 255, 0.92) !important;
  box-shadow: 0 -1px 0 rgba(0, 0, 0, 0.08);
}
html.dark .player-bar {
  /* 暗色：深色半透明 + 淡紫顶描边 */
  background: rgba(28, 28, 34, 0.92) !important;
  box-shadow: 0 -1px 0 color-mix(in srgb, var(--accent-light) 35%, transparent);
}

.bar-glow {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: color-mix(in srgb, var(--accent) 40%, transparent);
}

.player-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  height: 100%;
  gap: 24px;
  padding: 0 24px;
}

/* ---------- 左侧：歌曲信息 ---------- */
.player-track {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
  cursor: pointer;
  border-radius: 10px;
  padding: 6px 8px;
  margin: -6px -8px;
  transition: background-color 0.2s ease;
}

.player-track:hover {
  background: color-mix(in srgb, var(--n-text-color) 6%, transparent);
}

.track-cover {
  width: 52px;
  height: 52px;
  border-radius: 10px;
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.18);
  flex-shrink: 0;
  transition: transform 0.25s ease, box-shadow 0.25s ease;
}

.track-cover:hover {
  transform: scale(1.04);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--accent) 35%, transparent);
}

.track-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.track-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--n-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.track-artist {
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.like-btn {
  flex-shrink: 0;
  opacity: 0.7;
  transition: opacity 0.2s, transform 0.2s;
}

.like-btn:hover {
  opacity: 1;
  transform: scale(1.1);
}

/* ---------- 中间：控制 + 进度 ---------- */
.player-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 360px;
}

.player-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ctrl-btn {
  color: var(--n-text-color-3);
  transition: color 0.2s, transform 0.2s;
}

/* 点击后按钮保留焦点，naive-ui :focus 会套用主题色（按下后颜色不恢复），
   !important 压过 naive-ui 规则恢复常态色；悬停规则在后，悬停时正常高亮 */
.ctrl-btn:focus {
  color: var(--n-text-color-3) !important;
}

.ctrl-btn:hover,
.ctrl-btn:focus:hover {
  color: var(--n-text-color) !important;
  transform: scale(1.08);
}

/* 播放模式按钮：始终高亮，形态区分（参考网易云音乐） */
.mode-btn {
  color: var(--accent) !important;
}

.mode-btn:hover {
  color: #764ba2 !important;
}

.mode-tip {
  text-align: center;
  padding: 2px 0;
}

.mode-tip-name {
  font-size: 13px;
  font-weight: 600;
}

.mode-tip-next {
  font-size: 11px;
  opacity: 0.6;
  margin-top: 2px;
}

.repeat-one {
  position: relative;
  font-size: 9px;
  font-weight: 700;
  color: var(--accent);
  line-height: 1;
  margin-left: -6px;
  margin-top: -12px;
  align-self: flex-start;
}

.play-btn {
  width: 42px;
  height: 42px;
  /* 透明毛玻璃：无底色 + 模糊 + 紫色描边，图标改紫色（透明底不再用白色） */
  background: transparent !important;
  border: 1px solid color-mix(in srgb, var(--accent) 50%, transparent) !important;
  backdrop-filter: blur(12px) saturate(1.6);
  -webkit-backdrop-filter: blur(12px) saturate(1.6);
  color: var(--accent) !important;
  box-shadow: none;
  transition: transform 0.2s, box-shadow 0.2s;
}

html.dark .play-btn {
  border-color: color-mix(in srgb, var(--accent-light) 50%, transparent) !important;
  color: var(--accent-light) !important;
}

.play-btn:hover {
  transform: scale(1.06);
  box-shadow: 0 0 16px color-mix(in srgb, var(--accent) 35%, transparent);
}

.player-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.progress-slider {
  flex: 1;
  min-width: 0;
}

.time-label {
  font-size: 11px;
  color: var(--n-text-color-3);
  font-variant-numeric: tabular-nums;
  min-width: 34px;
  text-align: center;
}

/* ---------- 右侧：音量 + 操作 ---------- */
.player-right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 16px;
  min-width: 0;
}

.player-volume {
  display: flex;
  align-items: center;
  gap: 8px;
}

.vol-icon {
  color: var(--n-text-color-3);
  flex-shrink: 0;
  transition: color 0.2s;
}

.vol-icon:hover {
  color: var(--n-text-color);
}

.volume-slider {
  width: 80px;
}

/* ---------- 响应式 ---------- */
@media (max-width: 1100px) {
  .volume-slider {
    width: 60px;
  }
}

@media (max-width: 900px) {
  .player-grid {
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    gap: 4px 16px;
    padding: 8px 16px;
  }

  .player-center {
    grid-column: 1 / -1;
    grid-row: 1;
    order: 1;
    min-width: 0;
  }

  .player-track {
    grid-column: 1;
    grid-row: 2;
  }

  .player-right {
    grid-column: 2;
    grid-row: 2;
  }

  .volume-slider {
    width: 70px;
  }
}
</style>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon } from 'naive-ui'
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
  Headset,
  List,
} from '@vicons/ionicons5'
import { usePlayerStore } from '@/stores/player'
import PlaylistPanel from '@/components/Player/PlaylistPanel.vue'

const router = useRouter()
const player = usePlayerStore()

// 播放列表面板开关
const showPlaylistPanel = ref(false)

// 从共享 store 解构（保持响应式需通过 storeToRefs 或用 store.xxx）
const isPlaying = computed(() => player.isPlaying)
const progress = computed(() => player.progress)
const volume = computed(() => player.volume)
const isLiked = computed(() => player.isLiked)
const playMode = computed(() => player.playMode)
const currentTrack = computed(() => player.currentTrack)
const currentTime = computed(() => player.currentTime)
const modeTitle = computed(() => player.modeTitle)
const nextModeTitle = computed(() => player.nextModeTitle)

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

function goToPlayer() {
  router.push({ name: 'Player' })
}

function cyclePlayMode() {
  player.cyclePlayMode()
}

function onProgressChange(value: number) {
  player.seek(value)
}

function onVolumeChange(value: number) {
  player.setVolume(value)
}
</script>