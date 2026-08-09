<template>
  <div class="settings-page">
    <div class="settings-header">
      <n-h1 style="margin-bottom: 4px">设置</n-h1>
      <n-text depth="3">个性化你的 WBMusic 体验</n-text>
    </div>

    <div class="settings-layout">
      <!-- 左侧分类导航 -->
      <aside class="settings-nav">
        <div
          v-for="nav in navItems"
          :key="nav.key"
          class="nav-item"
          :class="{ active: activeSection === nav.key }"
          :style="activeSection === nav.key ? { backgroundColor: appStore.themeColor } : undefined"
          @click="activeSection = nav.key"
        >
          <n-icon :component="nav.icon" size="18" />
          <span>{{ nav.label }}</span>
        </div>
      </aside>

      <!-- 右侧内容 -->
      <main class="settings-main">
        <!-- 外观 -->
        <section v-show="activeSection === 'appearance'">
          <n-card title="外观" class="settings-card">
            <n-space vertical size="medium">
              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">主题模式</span>
                  <span class="setting-desc">跟随系统自动切换，或手动选择浅色 / 深色</span>
                </div>
                <n-radio-group v-model:value="themeMode">
                  <n-radio-button :value="'system'">跟随系统</n-radio-button>
                  <n-radio-button :value="'light'">浅色</n-radio-button>
                  <n-radio-button :value="'dark'">深色</n-radio-button>
                </n-radio-group>
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">主题色</span>
                  <span class="setting-desc">选择你的个性主色调</span>
                </div>
                <div class="theme-colors">
                  <div
                    v-for="c in themeColorOptions"
                    :key="c.value"
                    class="theme-dot"
                    :style="{ background: c.value }"
                    :class="{ active: appStore.themeColor === c.value }"
                    :title="c.label"
                    @click="handleThemeColor(c.value)"
                  ></div>
                </div>
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">侧边栏</span>
                  <span class="setting-desc">折叠侧边栏以腾出更多空间</span>
                </div>
                <n-switch
                  :value="appStore.sidebarCollapsed"
                  @update:value="handleSidebar"
                />
              </div>
            </n-space>
          </n-card>
        </section>

        <!-- 播放器 -->
        <section v-show="activeSection === 'player'">
          <n-card title="播放器" class="settings-card">
            <n-space vertical size="medium">
              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">默认播放模式</span>
                  <span class="setting-desc">歌曲的默认循环方式</span>
                </div>
                <n-select
                  v-model:value="playMode"
                  :options="playModeOptions"
                  style="width: 140px"
                />
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">默认音量</span>
                  <span class="setting-desc">新播放会话的初始音量</span>
                </div>
                <n-slider
                  v-model:value="volume"
                  :step="1"
                  :tooltip="false"
                  style="width: 200px"
                />
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">自动播放</span>
                  <span class="setting-desc">切换歌曲后自动继续播放</span>
                </div>
                <n-switch v-model:value="appStore.autoPlay" />
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">显示歌词</span>
                  <span class="setting-desc">播放页展示滚动歌词</span>
                </div>
                <n-switch v-model:value="appStore.showLyrics" />
              </div>
            </n-space>
          </n-card>
        </section>

        <!-- 存储与更新 -->
        <section v-show="activeSection === 'storage'">
          <n-card title="存储与更新" class="settings-card">
            <n-space vertical size="medium">
              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">缓存存放路径</span>
                  <span class="setting-desc">自定义缓存文件的保存位置</span>
                </div>
                <div class="cache-path-control">
                  <span class="cache-path-text" :title="appStore.cachePath">{{ appStore.cachePath }}</span>
                  <n-button size="small" secondary @click="openCachePathModal">修改</n-button>
                </div>
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">清除缓存</span>
                  <span class="setting-desc">清除本地缓存数据</span>
                </div>
                <n-button size="small" secondary @click="clearCache">清除</n-button>
              </div>

              <n-divider style="margin: 4px 0" />

              <div class="setting-row">
                <div class="setting-info">
                  <span class="setting-label">检查更新</span>
                  <span class="setting-desc">查找最新版本</span>
                </div>
                <n-button size="small" secondary @click="checkUpdate">检查</n-button>
              </div>
            </n-space>
          </n-card>
        </section>

        <!-- 关于 -->
        <section v-show="activeSection === 'about'">
          <n-card title="关于" class="settings-card">
            <div class="about-row">
              <n-avatar round size="medium" class="about-logo">W</n-avatar>
              <div class="about-info">
                <span class="about-name">WBMusic</span>
                <n-text depth="3" style="font-size: 12px">v0.1.3 · 音乐播放器应用</n-text>
              </div>
            </div>
          </n-card>
        </section>
      </main>
    </div>

    <!-- 缓存路径修改弹窗 -->
    <n-modal
      v-model:show="showCachePathModal"
      preset="card"
      title="缓存存放路径"
      style="width: 460px"
      :mask-closable="false"
    >
      <n-form label-placement="top">
        <n-form-item label="路径">
          <n-input
            v-model:value="cachePathInput"
            placeholder="例如：C:\\Users\\你的用户名\\AppData\\Local\\com.wbmusic.app\\cache"
            @keyup.enter="saveCachePath"
          />
        </n-form-item>
        <n-text depth="3" style="font-size: 12px">
          可以输入完整绝对路径，或相对路径（相对应用缓存目录）。支持字母、数字、中文、下划线与斜杠。
        </n-text>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showCachePathModal = false">取消</n-button>
          <n-button type="primary" @click="saveCachePath">保存</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, type Component } from 'vue'
import { useMessage } from 'naive-ui'
import { useAppStore, type ThemeMode } from '@/stores/app'
import { usePlayerStore } from '@/stores/player'
import { ColorPalette, Options, Server, InformationCircle } from '@vicons/ionicons5'

const appStore = useAppStore()
const player = usePlayerStore()
const message = useMessage()

type SectionKey = 'appearance' | 'player' | 'storage' | 'about'

const activeSection = ref<SectionKey>('appearance')

const navItems: { key: SectionKey; label: string; icon: Component }[] = [
  { key: 'appearance', label: '外观', icon: ColorPalette },
  { key: 'player', label: '播放器', icon: Options },
  { key: 'storage', label: '存储与更新', icon: Server },
  { key: 'about', label: '关于', icon: InformationCircle },
]

/** 主题模式：双向绑定 app store（持久化） */
const themeMode = computed<ThemeMode>({
  get: () => appStore.themeMode,
  set: (v) => appStore.setThemeMode(v),
})

/** 默认播放模式：绑定真实播放器状态（持久化） */
const playMode = computed({
  get: () => player.playMode,
  set: (v) => player.setPlayMode(v),
})

/** 默认音量：绑定真实播放器音量（持久化） */
const volume = computed({
  get: () => player.volume,
  set: (v) => player.setVolume(v),
})

const playModeOptions = [
  { label: '列表循环', value: 'list' },
  { label: '单曲循环', value: 'one' },
  { label: '随机播放', value: 'shuffle' },
]

const themeColorOptions = [
  { label: '品牌紫', value: '#667eea' },
  { label: '经典蓝', value: '#2f54eb' },
  { label: '云音乐红', value: '#d43c33' },
  { label: 'QQ 绿', value: '#31c27c' },
  { label: '活力橙', value: '#fa8c16' },
  { label: '少女粉', value: '#eb2f96' },
]

function handleThemeColor(color: string) {
  appStore.setThemeColor(color)
  message.success('主题色已切换')
}

function handleSidebar(value: boolean) {
  appStore.sidebarCollapsed = value
  message.info(value ? '侧边栏已折叠' : '侧边栏已展开')
}

function clearCache() {
  message.success('缓存已清除')
}

function checkUpdate() {
  message.info('当前已是最新版本 v0.1.3')
}

const showCachePathModal = ref(false)
const cachePathInput = ref('')

function openCachePathModal() {
  cachePathInput.value = appStore.cachePath
  showCachePathModal.value = true
}

function saveCachePath() {
  const path = cachePathInput.value.trim()
  if (!path) {
    message.warning('路径不能为空')
    return
  }
  if (!/^[\w\u4e00-\u9fa5/\\:-]+$/.test(path)) {
    message.warning('路径包含不支持的字符')
    return
  }
  appStore.setCachePath(path)
  showCachePathModal.value = false
  message.success('缓存路径已更新')
}
</script>

<style scoped>
.settings-page {
  max-width: 1100px;
}

.settings-header {
  margin-bottom: 24px;
}

/* ===== 两栏布局 ===== */
.settings-layout {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}

.settings-nav {
  width: 220px;
  flex-shrink: 0;
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 12px;
  padding: 8px;
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  color: var(--n-text-color-2);
  transition: background 0.15s, color 0.15s;
  user-select: none;
}

.nav-item:hover {
  background: var(--n-color-2);
  color: var(--n-text-color);
}

.nav-item.active {
  color: #fff;
}

.settings-main {
  flex: 1;
  min-width: 0;
}

@media (max-width: 900px) {
  .settings-layout {
    flex-direction: column;
  }

  .settings-nav {
    width: 100%;
    flex-direction: row;
    overflow-x: auto;
    position: static;
  }

  .nav-item {
    white-space: nowrap;
    flex-shrink: 0;
  }
}

.settings-card :deep(.n-card-header) {
  font-weight: 600;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.setting-label {
  font-size: 14px;
  font-weight: 500;
}

.setting-desc {
  font-size: 12px;
  color: var(--n-text-color-3);
}

/* 主题色选择 */
.theme-colors {
  display: flex;
  align-items: center;
  gap: 10px;
}

.theme-dot {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid transparent;
  transition: transform 0.2s, box-shadow 0.2s;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.theme-dot:hover {
  transform: scale(1.15);
}

.theme-dot.active {
  border-color: var(--n-text-color);
  transform: scale(1.15);
}

/* 缓存路径 */
.cache-path-control {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.cache-path-text {
  font-size: 13px;
  font-family: 'Consolas', 'Courier New', monospace;
  color: var(--n-text-color-2);
  max-width: 340px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: ltr;
  text-align: right;
}

/* 关于 */
.about-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.about-logo {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-weight: 700;
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about-name {
  font-size: 15px;
  font-weight: 600;
}
</style>
