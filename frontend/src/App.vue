<template>
  <n-config-provider
    :theme="appStore.isDark ? darkTheme : lightTheme"
    :theme-overrides="themeOverrides"
  >
    <n-message-provider>
      <n-notification-provider>
        <router-view />
      </n-notification-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  NConfigProvider,
  NMessageProvider,
  NNotificationProvider,
  lightTheme,
  darkTheme,
  type GlobalThemeOverrides,
} from 'naive-ui'
import { useAppStore } from '@/stores/app'

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
</script>
