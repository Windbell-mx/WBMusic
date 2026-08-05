import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, 'src'),
    },
  },
  server: {
    // 忽略 Rust 构建产物目录，避免 vite 监视被 cargo 锁定的文件导致 EBUSY
    watch: {
      ignored: ['**/src-tauri/target/**', '**/target/**'],
    },
  },
})
