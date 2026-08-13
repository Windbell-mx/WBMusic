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
    // 忽略 Rust 构建产物目录，避免 vite 监视被 cargo 锁定的文件导致 EBUSY
    watch: {
      ignored: ['**/src-tauri/target/**', '**/target/**'],
    },
  },
})
