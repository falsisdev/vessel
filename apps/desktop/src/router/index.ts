import { createRouter, createWebHashHistory } from 'vue-router'
import DetailsView from '@/views/DetailsView.vue'
import DownloadsView from '@/views/DownloadsView.vue'
import ExtensionsView from '@/views/ExtensionsView.vue'
import HomeView from '@/views/HomeView.vue'
import IptvView from '@/views/IptvView.vue'
import LibraryView from '@/views/LibraryView.vue'
import PlayerView from '@/views/PlayerView.vue'
import SearchView from '@/views/SearchView.vue'
import SettingsView from '@/views/SettingsView.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: HomeView },
    { path: '/search', name: 'search', component: SearchView },
    {
      path: '/details/:pluginId/:nativeId',
      name: 'details',
      component: DetailsView,
      props: true,
    },
    {
      path: '/watch/:pluginId/:nativeId',
      name: 'player',
      component: PlayerView,
      props: true,
    },
    { path: '/library', name: 'library', component: LibraryView },
    { path: '/iptv', name: 'iptv', component: IptvView },
    { path: '/downloads', name: 'downloads', component: DownloadsView },
    { path: '/extensions', name: 'extensions', component: ExtensionsView },
    { path: '/settings', name: 'settings', component: SettingsView },
  ],
})
