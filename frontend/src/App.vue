<template>
  <n-config-provider
    :theme="appStore.isDark ? darkTheme : lightTheme"
    :theme-overrides="themeOverrides"
  >
    <!-- 自定义标题栏（隐藏系统标题栏后用于拖拽 + 最小化/最大化/关闭） -->
    <TitleBar />
    <n-message-provider>
      <n-notification-provider>
        <router-view />
      </n-notification-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import {
  NConfigProvider,
  NMessageProvider,
  NNotificationProvider,
  lightTheme,
  darkTheme,
  type GlobalThemeOverrides,
} from 'naive-ui'
import { useAppStore } from '@/stores/app'
import TitleBar from '@/components/TitleBar.vue'

const appStore = useAppStore()

// 根据 store 中的主题色动态生成 Naive UI 主题覆盖
const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const primary = appStore.themeColor
  return {
    common: {
      primaryColor: primary,
      primaryColorHover: primary + 'cc',
      primaryColorPressed: primary + 'd9',
      primaryColorSuppl: primary + 'e6',
    },
  }
})

// 主题色同步为全局 CSS 变量（--accent / --accent-light），
// 供自定义组件（标题栏、侧边栏、播放器等拿不到 naive 变量的元素）使用
watch(
  () => appStore.themeColor,
  (color) => {
    const root = document.documentElement
    root.style.setProperty('--accent', color)
    // 暗色下使用的亮化版主题色（与白色混合提亮）
    root.style.setProperty('--accent-light', `color-mix(in srgb, ${color} 55%, #ffffff)`)
  },
  { immediate: true },
)
</script>
