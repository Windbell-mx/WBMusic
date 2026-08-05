<template>
  <n-modal
    :show="show"
    preset="card"
    :title="`登录 ${provider?.label ?? ''}`"
    style="width: 480px"
    :mask-closable="false"
    @update:show="handleClose"
  >
    <n-space vertical size="large">
      <!-- 当前登录状态 -->
      <n-alert v-if="currentStatus?.logged_in" type="success" :bordered="false">
        <n-space align="center" justify="space-between">
          <span>
            当前已登录：{{ currentStatus.nickname || currentStatus.user_id || '未知用户' }}
          </span>
          <n-button size="tiny" secondary @click="handleLogout">退出登录</n-button>
        </n-space>
      </n-alert>

      <n-tabs v-model:value="activeTab" type="line" animated>
        <!-- 扫码登录 -->
        <n-tab-pane name="qr" tab="扫码登录">
          <n-alert type="info" :bordered="false" style="margin-bottom: 12px">
            将打开 <b>{{ provider?.label }}</b> 官方登录窗口。
            <template v-if="isQq">窗口内支持 <b>QQ</b> 与 <b>微信</b> 两种扫码方式，可自由切换。</template>
            请使用手机 App 扫码并确认授权，登录成功后自动完成。
          </n-alert>
          <n-space justify="center" style="padding: 20px 0">
            <n-button type="primary" size="large" :loading="openingQr" @click="handleOpenQr">
              <template #icon><n-icon :component="QrCode" /></template>
              打开扫码登录窗口
            </n-button>
          </n-space>
          <n-text depth="3" style="font-size: 12px">
            提示：登录窗口为官方登录页面，扫码授权后应用仅读取必要的登录 Cookie 并保存在本机系统凭据库，不会上传。
          </n-text>
        </n-tab-pane>

        <!-- Cookie 粘贴登录（兜底） -->
        <n-tab-pane name="cookie" tab="Cookie 粘贴">
          <n-alert type="info" :bordered="false">
            在浏览器中登录 {{ provider?.label }} 官网，打开开发者工具（F12）→ Network，
            复制任意请求的 <b>Cookie</b> 粘贴到下方即可完成登录。登录凭据仅保存在本机，不会上传。
          </n-alert>
          <n-form label-placement="top" style="margin-top: 12px">
            <n-form-item label="Cookie">
              <n-input
                v-model:value="loginCookie"
                type="textarea"
                :rows="5"
                placeholder="粘贴你的 Cookie，例如：MUSIC_U=xxx; __csrf=xxx"
              />
            </n-form-item>
          </n-form>
          <n-text depth="3" style="font-size: 12px">
            提示：当扫码登录不可用时，可使用 Cookie 粘贴登录作为备用方式。
          </n-text>
        </n-tab-pane>
      </n-tabs>
    </n-space>
    <template #footer>
      <n-space justify="end">
        <n-button @click="handleClose">取消</n-button>
        <n-button
          v-if="activeTab === 'cookie'"
          type="primary"
          :loading="loggingIn"
          :disabled="!loginCookie.trim()"
          @click="handleLogin"
        >
          登录
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { listen } from '@tauri-apps/api/event'
import { QrCode } from '@vicons/ionicons5'
import {
  getLoginStatus,
  isTauri,
  loginMusic,
  logoutMusic,
  openQrLogin,
  type LoginStatus,
  type MusicSource,
} from '@/api'

const props = defineProps<{
  show: boolean
  provider: { key: MusicSource; label: string } | null
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'changed'): void
}>()

const message = useMessage()
const loginCookie = ref('')
const loggingIn = ref(false)
const openingQr = ref(false)
const activeTab = ref<'qr' | 'cookie'>('qr')
const currentStatus = ref<LoginStatus | null>(null)

const isQq = computed(() => props.provider?.key === 'qq_music')

// 扫码登录成功事件（由后端 poll_qr_login 轮询捕获后触发）
let unlistenQr: (() => void) | null = null

// 每次打开弹窗时刷新当前登录状态
watch(
  () => [props.show, props.provider?.key] as const,
  async () => {
    if (props.show && props.provider) {
      loginCookie.value = ''
      activeTab.value = 'qr'
      try {
        const statuses = await getLoginStatus()
        currentStatus.value = statuses[props.provider.key] ?? null
      } catch {
        currentStatus.value = null
      }
      // 注册扫码成功事件监听（仅 Tauri 环境）
      if (isTauri && !unlistenQr) {
        try {
          unlistenQr = await listen<LoginStatus>('qr-login-success', (event) => {
            if (props.provider) {
              currentStatus.value = event.payload
              message.success(`已登录 ${props.provider.label}`)
              emit('changed')
              emit('update:show', false)
            }
          })
        } catch {
          /* 事件监听失败不影响使用 */
        }
      }
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  unlistenQr?.()
})

async function handleOpenQr() {
  if (!props.provider) return
  openingQr.value = true
  try {
    await openQrLogin(props.provider.key)
    message.info('已打开扫码登录窗口，请在弹出的窗口中扫码')
  } catch (e) {
    message.error(`打开登录窗口失败: ${e}`)
  } finally {
    openingQr.value = false
  }
}

function handleClose() {
  emit('update:show', false)
}

async function handleLogin() {
  if (!props.provider || !loginCookie.value.trim()) return
  loggingIn.value = true
  try {
    const status = await loginMusic(props.provider.key, {
      type: 'cookie',
      value: loginCookie.value.trim(),
    })
    currentStatus.value = status
    loginCookie.value = ''
    message.success(`已登录 ${props.provider.label}`)
    emit('changed')
    emit('update:show', false)
  } catch (e) {
    message.error(`登录失败: ${e}`)
  } finally {
    loggingIn.value = false
  }
}

async function handleLogout() {
  if (!props.provider) return
  try {
    await logoutMusic(props.provider.key)
    currentStatus.value = { logged_in: false }
    message.success(`已退出 ${props.provider.label}`)
    emit('changed')
    // 退出成功后直接关闭登录卡片
    emit('update:show', false)
  } catch (e) {
    message.error(`退出失败: ${e}`)
  }
}
</script>
