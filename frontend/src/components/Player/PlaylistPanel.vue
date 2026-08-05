<template>
  <transition name="pl-slide">
    <div v-if="visible" class="pl-overlay" @click.self="close">
      <div class="pl-panel">
        <!-- 头部 -->
        <div class="pl-header">
          <div class="pl-title">
            <n-icon :component="List" size="18" />
            <span>播放列表</span>
            <span class="pl-count">{{ playlist.length }} 首</span>
          </div>
          <div class="pl-actions">
            <n-tooltip trigger="hover" placement="top">
              <template #trigger>
                <n-button text class="pl-action" @click="cyclePlayMode">
                  <n-icon :component="playMode === 'shuffle' ? Shuffle : Repeat" size="18" />
                  <span v-if="playMode === 'one'" class="repeat-one">1</span>
                </n-button>
              </template>
              <span>播放模式：{{ modeTitle }}（点击切换 {{ nextModeTitle }}）</span>
            </n-tooltip>
            <n-tooltip trigger="hover" placement="top">
              <template #trigger>
                <n-button text class="pl-action" @click="clearList">
                  <n-icon :component="TrashOutline" size="18" />
                </n-button>
              </template>
              <span>清空列表</span>
            </n-tooltip>
            <n-button text class="pl-action" @click="close" title="收起">
              <n-icon :component="ChevronDown" size="20" />
            </n-button>
          </div>
        </div>

        <!-- 列表 -->
        <div class="pl-list" v-if="playlist.length > 0">
          <div
            v-for="(track, index) in playlist"
            :key="`${track.source}-${track.id}`"
            class="pl-item"
            :class="{ active: index === currentIndex }"
            @click="playAt(index)"
          >
            <span class="pl-idx">
              <n-icon v-if="index === currentIndex && isPlaying" :component="VolumeHigh" size="14" class="eq-icon" />
              <template v-else>{{ index === currentIndex ? '♪' : index + 1 }}</template>
            </span>
            <div class="pl-info">
              <div class="pl-name">{{ track.title }}</div>
              <div class="pl-artist">{{ track.artist }}</div>
            </div>
            <span class="pl-duration">{{ formatDuration(track.duration) }}</span>
            <n-button text class="pl-remove" @click.stop="removeAt(index)" title="从列表移除">
              <n-icon :component="Close" size="14" />
            </n-button>
          </div>
        </div>

        <!-- 空状态 -->
        <div v-else class="pl-empty">
          <n-icon :component="MusicalNotesOutline" size="40" :depth="3" />
          <n-text depth="3">播放列表为空，去歌单里挑一首吧</n-text>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NIcon, NButton, NText, NTooltip } from 'naive-ui'
import {
  List,
  Shuffle,
  Repeat,
  TrashOutline,
  ChevronDown,
  Close,
  VolumeHigh,
  MusicalNotesOutline,
} from '@vicons/ionicons5'
import { usePlayerStore } from '@/stores/player'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ (e: 'update:visible', v: boolean): void }>()

const player = usePlayerStore()

const playlist = computed(() => player.playlist)
const currentIndex = computed(() => player.currentIndex)
const isPlaying = computed(() => player.isPlaying)
const playMode = computed(() => player.playMode)
const modeTitle = computed(() => player.modeTitle)
const nextModeTitle = computed(() => player.nextModeTitle)

function close() {
  emit('update:visible', false)
}

function playAt(index: number) {
  const track = playlist.value[index]
  if (!track) return
  if (index === currentIndex.value) {
    // 点击当前曲目：切换播放/暂停
    player.togglePlay()
    return
  }
  player.playTrack(track, index)
}

function removeAt(index: number) {
  player.removeTrack(index)
}

function clearList() {
  player.clearPlaylist()
}

function cyclePlayMode() {
  player.cyclePlayMode()
}

/** 秒 → mm:ss（兼容无时长） */
function formatDuration(sec: number | null | undefined): string {
  if (!sec || sec <= 0) return '--:--'
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  return `${m}:${String(s).padStart(2, '0')}`
}
</script>

<style scoped>
.pl-overlay {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  justify-content: flex-end;
  background: rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
}

.pl-panel {
  width: 380px;
  max-width: 90vw;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--n-color);
  box-shadow: -12px 0 32px rgba(0, 0, 0, 0.18);
}

/* 头部 */
.pl-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 16px 14px;
  border-bottom: 1px solid var(--n-border-color);
}

.pl-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
}

.pl-count {
  font-size: 12px;
  font-weight: 400;
  color: var(--n-text-color-3);
  background: var(--n-color-2);
  padding: 2px 8px;
  border-radius: 10px;
}

.pl-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.pl-action {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  transition: background 0.2s;
}

.pl-action:hover {
  background: var(--n-color-2);
}

.repeat-one {
  position: absolute;
  font-size: 9px;
  font-weight: 700;
  line-height: 1;
}

/* 列表 */
.pl-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.pl-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 16px;
  cursor: pointer;
  transition: background 0.15s;
}

.pl-item:hover {
  background: var(--n-color-2);
}

.pl-item.active {
  background: color-mix(in srgb, var(--n-color-2) 85%, transparent);
}

.pl-item.active .pl-name {
  color: var(--primary-color, #667eea);
  font-weight: 600;
}

.pl-idx {
  width: 22px;
  flex-shrink: 0;
  text-align: center;
  font-size: 13px;
  color: var(--n-text-color-3);
  display: flex;
  align-items: center;
  justify-content: center;
}

.pl-item.active .pl-idx {
  color: var(--primary-color, #667eea);
}

.eq-icon {
  animation: eq-bounce 0.9s ease-in-out infinite;
}

@keyframes eq-bounce {
  0%, 100% { transform: scaleY(0.6); }
  50% { transform: scaleY(1.1); }
}

.pl-info {
  flex: 1;
  min-width: 0;
}

.pl-name {
  font-size: 14px;
  color: var(--n-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pl-artist {
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 1px;
}

.pl-duration {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--n-text-color-3);
}

.pl-remove {
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.15s;
}

.pl-item:hover .pl-remove {
  opacity: 1;
}

/* 空状态 */
.pl-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 40px;
}

/* 过渡动画 */
.pl-slide-enter-active,
.pl-slide-leave-active {
  transition: opacity 0.25s ease;
}

.pl-slide-enter-active .pl-panel,
.pl-slide-leave-active .pl-panel {
  transition: transform 0.25s ease;
}

.pl-slide-enter-from,
.pl-slide-leave-to {
  opacity: 0;
}

.pl-slide-enter-from .pl-panel,
.pl-slide-leave-to .pl-panel {
  transform: translateX(100%);
}
</style>
