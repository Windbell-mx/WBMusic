import { createApp } from 'vue'
import { createPinia } from 'pinia'
import NaiveUI from 'naive-ui'
import App from './App.vue'
import router from './router'
import { isTauri } from './api'
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
app.use(NaiveUI)

app.mount('#app')
