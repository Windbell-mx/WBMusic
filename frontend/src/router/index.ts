import { createRouter, createWebHashHistory } from 'vue-router'
import DefaultLayout from '@/layouts/DefaultLayout.vue'

const router = createRouter({
  // Tauri 打包后基于本地文件协议加载，必须使用 hash 模式
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      component: DefaultLayout,
      children: [
        {
          path: '',
          name: 'Home',
          component: () => import('@/views/HomeView.vue'),
        },
        {
          path: 'playlist',
          name: 'Playlist',
          component: () => import('@/views/Playlist/PlaylistView.vue'),
        },
        {
          path: 'search',
          name: 'Search',
          component: () => import('@/views/SearchView.vue'),
        },
        {
          path: 'playlist/:id',
          name: 'PlaylistDetail',
          component: () => import('@/views/Playlist/PlaylistDetailView.vue'),
        },
        {
          path: 'settings',
          name: 'Settings',
          component: () => import('@/views/SettingsView.vue'),
        },
        {
          path: 'player',
          name: 'Player',
          component: () => import('@/views/Player/PlayerView.vue'),
        },
      ],
    },
  ],
})

export default router
