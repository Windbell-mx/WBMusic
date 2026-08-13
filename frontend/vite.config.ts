import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import { NaiveUiResolver } from 'unplugin-vue-components/resolvers'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    // naive-ui 按需引入：模板中的 <n-xxx> 自动解析，避免全量打包（主包 968KB → 大幅缩小）
    Components({
      resolvers: [NaiveUiResolver()],
      dts: 'src/components.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, 'src'),
    },
  },
  server: {
    // 固定开发端口：5173 留给浏览器调试页（localhost:5173），
    // Tauri 应用固定用 5174（tauri.conf.json 的 devUrl 已同步）。
    // strictPort 保证端口被占用时报错而非静默跳转，避免 Tauri 窗口加载失败
    port: 5174,
    strictPort: true,
    // 忽略 Rust 构建产物目录，避免 vite 监视被 cargo 锁定的文件导致 EBUSY
    watch: {
      ignored: ['**/src-tauri/target/**', '**/target/**'],
    },
  },
})
