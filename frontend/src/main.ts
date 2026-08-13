import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { isTauri } from './api'
import { useShortcutStore } from './stores/shortcuts'
import './style.css'

// 桌面应用内禁用 WebView2 默认右键菜单（"检查/刷新/另存为"等浏览器菜单）
// 原理：contextmenu 事件 preventDefault 后 WebView2 不显示默认菜单
// 仅 Tauri 环境生效；浏览器调试时保留原生右键。
// 输入框/文本域放行，保留复制粘贴等常用右键功能。
if (isTauri) {
  window.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement | null
    if (target && (target.closest('input') || target.closest('textarea') || target.isContentEditable)) {
      return
    }
    e.preventDefault()
  })
}

const app = createApp(App)

app.use(createPinia())
app.use(router)
// naive-ui 组件已由 unplugin-vue-components 按需自动引入，无需全量注册

// 初始化全局快捷键（注册 window keydown 监听；即使从未打开设置页也生效）
useShortcutStore()

app.mount('#app')
