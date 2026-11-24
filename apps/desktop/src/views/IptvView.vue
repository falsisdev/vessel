<script setup lang="ts">
import { computed, onActivated, onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { Package, Play, Radio, RefreshCw, Star, Trash2, Tv } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import IptvProfileForm from '@/components/iptv/IptvProfileForm.vue'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { pushToast } from '@/composables/useVesselEvents'
import type {
  EpgNowNext,
  HomeSection,
  IptvKind,
  MediaSummary,
  PluginListEntry,
  StoredIptvChannel,
  StoredIptvProfile,
  StreamCandidate,
} from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()

const activeTab = shallowRef<'playlists' | 'plugins'>('plugins')
const livePlugins = ref<PluginListEntry[]>([])
const selectedLivePluginId = shallowRef<string>('')
const pluginLiveSections = ref<HomeSection[]>([])
const loadingPluginLive = shallowRef(false)
const pluginChannelFilter = shallowRef('')
const launchingChannelId = shallowRef('')
const failedPluginLogos = ref<Record<string, boolean>>({})

const profiles = ref<StoredIptvProfile[]>([])
const activeProfileId = shallowRef<string>('')
const groups = ref<string[]>([])
const selectedGroup = shallowRef<string>('')
const searchQuery = shallowRef('')
const favoritesOnly = shallowRef(false)
const channels = ref<StoredIptvChannel[]>([])
const selectedChannel = ref<StoredIptvChannel | null>(null)
const epg = ref<EpgNowNext | null>(null)
const syncingProfileId = shallowRef<string>('')
const failedLogos = ref<Record<number, boolean>>({})

function resolveLiveGroupName(entry: PluginListEntry): string {
  if (entry.repoId?.trim()) return entry.repoId.trim()
  const domains = entry.allowedDomains ?? []
  if (
    domains.includes('raw.githubusercontent.com') &&
    (entry.manifest.author === 'falsisdev' ||
      entry.manifest.author === 'metemel' ||
      entry.manifest.id.startsWith('anthology') ||
      entry.manifest.id === 'm3u.anthology.addon')
  ) {
    return 'Anthology'
  }
  if (entry.manifest.id.startsWith('community.anthology')) return 'Anthology'
  if (entry.sourceType === 'dev_dir') return 'Local Dev Plugins'
  return 'Standalone Plugins'
}

function isLiveTvPlugin(p: PluginListEntry): boolean {
  if (!p.enabled) return false
  const caps = p.manifest.capabilities ?? []
  if (caps.includes('live_tv')) return true
  const hay = `${p.manifest.id} ${p.manifest.name} ${p.manifest.description ?? ''}`.toLowerCase()
  return hay.includes('iptv') || hay.includes('canlı tv') || hay.includes('canli tv') || hay.includes('live tv') || hay.includes('_spor')
}

const groupedLivePlugins = computed(() => {
  const buckets = new Map<string, PluginListEntry[]>()
  for (const p of livePlugins.value) {
    const group = resolveLiveGroupName(p)
    const arr = buckets.get(group)
    if (arr) arr.push(p)
    else buckets.set(group, [p])
  }
  return Array.from(buckets.entries()).map(([name, items]) => ({ name, items }))
})

const filteredPluginLiveSections = computed(() => {
  const q = pluginChannelFilter.value.trim().toLowerCase()
  return pluginLiveSections.value
    .map((sec) => ({
      ...sec,
      items: q ? sec.items.filter((it) => it.title.toLowerCase().includes(q)) : sec.items,
    }))
    .filter((sec) => sec.items.length > 0)
})

async function refreshLivePlugins(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    const all = await vesselApi.listPlugins()
    livePlugins.value = all.filter(isLiveTvPlugin)
    if (
      livePlugins.value.length &&
      (!selectedLivePluginId.value ||
        !livePlugins.value.some((p) => p.manifest.id === selectedLivePluginId.value))
    ) {
      selectedLivePluginId.value = livePlugins.value[0].manifest.id
    }
  } catch {
    livePlugins.value = []
  }
}

async function selectLivePlugin(pluginId: string): Promise<void> {
  selectedLivePluginId.value = pluginId
  if (!pluginId || !isTauriRuntime()) return
  loadingPluginLive.value = true
  try {
    const res = await vesselApi.getHomeFeeds([pluginId])
    const feed = res[0]
    if (feed?.error) {
      pushToast('warn', `${feed.pluginName}: ${feed.error}`)
    }
    pluginLiveSections.value = feed?.sections ?? []
  } catch (err) {
    pushToast('error', String(err))
    pluginLiveSections.value = []
  } finally {
    loadingPluginLive.value = false
  }
}

async function switchIptvTab(tab: 'playlists' | 'plugins'): Promise<void> {
  activeTab.value = tab
  if (tab === 'plugins') {
    await refreshLivePlugins()
    if (selectedLivePluginId.value && !pluginLiveSections.value.length) {
      await selectLivePlugin(selectedLivePluginId.value)
    }
  }
}

async function playPluginLiveItem(item: MediaSummary, sectionTitle: string): Promise<void> {
  const pluginId = item.pluginId || selectedLivePluginId.value
  if (!pluginId || launchingChannelId.value) return
  launchingChannelId.value = item.nativeId
  try {
    const detail = await vesselApi.getDetailsWithEnrichment(pluginId, item.nativeId, 'en')
    const ep = detail.episodes[0] ?? {
      streamId: item.nativeId,
      season: 1,
      episode: 1,
      title: item.title,
      overview: sectionTitle,
      thumbnail: item.poster,
      tags: [],
    }
    const streamRes = await vesselApi.getStreams(pluginId, ep.streamId)
    if (!streamRes.streams.length) {
      pushToast('warn', `No playable live streams returned for ${item.title}`)
      return
    }
    setSession({
      pluginId,
      nativeId: item.nativeId,
      detail: {
        ...detail,
        type: 'live',
        title: detail.title || item.title,
        poster: detail.poster ?? item.poster,
        backdrop: detail.backdrop ?? item.poster,
      },
      episode: ep,
      streams: streamRes.streams,
      subtitles: streamRes.subtitles ?? [],
      activeStreamIndex: 0,
    })
    await router.push({
      name: 'player',
      params: { pluginId, nativeId: item.nativeId },
      query: { streamId: ep.streamId },
    })
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    launchingChannelId.value = ''
  }
}

async function refreshProfiles(): Promise<void> {
  if (!isTauriRuntime()) return
  profiles.value = await vesselApi.listIptvProfiles()
  if (!activeProfileId.value && profiles.value.length) {
    activeProfileId.value = profiles.value[0].id
  }
  await refreshGroupsAndChannels()
}

async function refreshGroupsAndChannels(): Promise<void> {
  if (!isTauriRuntime()) return
  if (activeProfileId.value) {
    groups.value = await vesselApi.listIptvGroups(activeProfileId.value)
  } else {
    groups.value = []
  }
  await runChannelSearch()
}

async function runChannelSearch(): Promise<void> {
  if (!isTauriRuntime()) return
  channels.value = await vesselApi.searchIptvChannels({
    profileId: activeProfileId.value || undefined,
    query: searchQuery.value.trim() || undefined,
    groupTitle: selectedGroup.value || undefined,
    favoritesOnly: favoritesOnly.value,
    limit: 200,
  })
}

async function createProfile(payload: {
  id: string
  name: string
  kind: IptvKind
  serverUrl: string
  username?: string
  password?: string
  epgUrl?: string
  userAgent?: string
}): Promise<void> {
  try {
    const saved = await vesselApi.upsertIptvProfile(payload)
    activeProfileId.value = saved.id
    await refreshProfiles()
    pushToast('info', `Added IPTV profile ${saved.name} — syncing channels…`)
    await syncChannels(saved.id)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function syncChannels(profileId: string): Promise<void> {
  if (syncingProfileId.value) return
  syncingProfileId.value = profileId
  try {
    const count = await vesselApi.syncIptvProfile(profileId)
    failedLogos.value = {}
    await refreshGroupsAndChannels()
    pushToast('info', `Synced ${count} channels`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    syncingProfileId.value = ''
  }
}

async function syncEpg(profileId: string): Promise<void> {
  try {
    const count = await vesselApi.syncIptvEpg(profileId)
    pushToast('info', `Synced ${count} EPG slots`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function removeProfile(profileId: string): Promise<void> {
  try {
    await vesselApi.deleteIptvProfile(profileId)
    if (activeProfileId.value === profileId) {
      activeProfileId.value = ''
    }
    await refreshProfiles()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function selectChannel(ch: StoredIptvChannel): Promise<void> {
  selectedChannel.value = ch
  epg.value = null
  if (ch.tvgId) {
    try {
      const nowTs = Math.floor(Date.now() / 1000)
      epg.value = await vesselApi.getEpgNowNext(ch.profileId, ch.tvgId, nowTs)
    } catch {
      epg.value = null
    }
  }
}

async function toggleFavorite(ch: StoredIptvChannel): Promise<void> {
  try {
    await vesselApi.setIptvFavorite(ch.id, !ch.isFavorite)
    await runChannelSearch()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function playChannel(ch: StoredIptvChannel): Promise<void> {
  const buildChannelCandidates = (item: StoredIptvChannel): StreamCandidate[] => {
    const itemProf = profiles.value.find((p) => p.id === item.profileId)
    const itemHeaders: Record<string, string> = {}
    if (itemProf?.userAgent) itemHeaders['User-Agent'] = itemProf.userAgent

    const rawUrl = item.streamUrl
    const lowerUrl = rawUrl.toLowerCase()
    const isXtreamLive = item.streamId.startsWith('xtream-') && lowerUrl.includes('/live/')
    const list: StreamCandidate[] = []

    if (isXtreamLive && lowerUrl.endsWith('.ts')) {
      const hlsUrl = `${rawUrl.slice(0, -3)}.m3u8`
      list.push(
        {
          url: hlsUrl,
          kind: 'hls',
          label: `${item.name} (HLS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
        {
          url: rawUrl,
          kind: 'mpeg_ts',
          label: `${item.name} (MPEG-TS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
      )
    } else if (isXtreamLive && lowerUrl.endsWith('.m3u8')) {
      const tsUrl = `${rawUrl.slice(0, -5)}.ts`
      list.push(
        {
          url: rawUrl,
          kind: 'hls',
          label: `${item.name} (HLS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
        {
          url: tsUrl,
          kind: 'mpeg_ts',
          label: `${item.name} (MPEG-TS)`,
          flags: ['requires_proxy'],
          headers: itemHeaders,
          subtitles: [],
        },
      )
    } else {
      const isHls = lowerUrl.includes('.m3u8')
      list.push({
        url: rawUrl,
        kind: isHls ? 'hls' : 'mpeg_ts',
        label: isHls ? `${item.name} (HLS)` : `${item.name} (MPEG-TS)`,
        flags: ['requires_proxy'],
        headers: itemHeaders,
        subtitles: [],
      })
    }
    return list
  }

  const channelStreams: Record<string, StreamCandidate> = {}
  const episodes = channels.value.map((item, idx) => {
    const candidates = buildChannelCandidates(item)
    channelStreams[item.streamId] = candidates[0]
    return {
      streamId: item.streamId,
      season: 1,
      episode: idx + 1,
      title: item.name,
      overview: item.groupTitle || undefined,
      thumbnail: item.logoUrl || undefined,
      tags: [],
    }
  })
  const activeEp =
    episodes.find((e) => e.streamId === ch.streamId) ?? {
      streamId: ch.streamId,
      season: 1,
      episode: 1,
      title: ch.name,
      overview: ch.groupTitle || undefined,
      thumbnail: ch.logoUrl || undefined,
      tags: [],
    }
  const activeCandidates = buildChannelCandidates(ch)
  if (!channelStreams[ch.streamId]) {
    channelStreams[ch.streamId] = activeCandidates[0]
  }

  setSession({
    pluginId: 'iptv',
    nativeId: ch.streamId,
    detail: {
      nativeId: ch.streamId,
      pluginId: 'iptv',
      type: 'live',
      isAnime: false,
      title: ch.name,
      poster: ch.logoUrl || undefined,
      backdrop: ch.logoUrl || undefined,
      synopsis: epg.value?.now ? `Now: ${epg.value.now.title}` : ch.groupTitle || undefined,
      genres: ch.groupTitle ? [ch.groupTitle] : ['Live TV'],
      externalIds: {},
      episodes,
    },
    episode: activeEp,
    streams: activeCandidates,
    subtitles: [],
    activeStreamIndex: 0,
    channelStreams,
  })

  await router.push({
    name: 'player',
    params: { pluginId: 'iptv', nativeId: ch.streamId },
    query: { streamId: ch.streamId },
  })
}

onMounted(async () => {
  void refreshProfiles()
  await refreshLivePlugins()
  if (selectedLivePluginId.value && !pluginLiveSections.value.length) {
    await selectLivePlugin(selectedLivePluginId.value)
  }
})

onActivated(() => {
  void refreshLivePlugins()
})
</script>

<template>
  <section class="iptv-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('iptv.title') }}</h1>
      <div class="iptv-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          :aria-selected="activeTab === 'plugins'"
          class="v-btn"
          :class="{ 'v-btn-primary': activeTab === 'plugins' }"
          @click="switchIptvTab('plugins')"
        >
          <Radio :size="15" aria-hidden="true" />
          <span>{{ t('iptv.tab_plugins') }}</span>
          <span v-if="livePlugins.length" class="v-badge">{{ livePlugins.length }}</span>
        </button>
        <button
          type="button"
          role="tab"
          :aria-selected="activeTab === 'playlists'"
          class="v-btn"
          :class="{ 'v-btn-primary': activeTab === 'playlists' }"
          @click="switchIptvTab('playlists')"
        >
          <Tv :size="15" aria-hidden="true" />
          <span>{{ t('iptv.tab_playlists') }}</span>
        </button>
      </div>
    </header>

    <template v-if="activeTab === 'playlists'">
      <IptvProfileForm @submit="createProfile" />

    <div v-if="profiles.length" class="profiles-strip">
      <div
        v-for="prof in profiles"
        :key="prof.id"
        class="v-card profile-pill"
        :class="{ 'is-selected': activeProfileId === prof.id }"
      >
        <button class="profile-select" @click="activeProfileId = prof.id; refreshGroupsAndChannels()">
          <strong>{{ prof.name }}</strong>
          <span class="v-badge">{{ prof.kind }}</span>
        </button>
        <div class="profile-actions">
          <button
            class="v-btn"
            :disabled="syncingProfileId === prof.id"
            @click="syncChannels(prof.id)"
          >
            <RefreshCw
              :size="14"
              :class="{ spin: syncingProfileId === prof.id }"
              aria-hidden="true"
            />
            {{ syncingProfileId === prof.id ? 'Syncing…' : t('iptv.sync_channels') }}
          </button>
          <button class="v-btn" @click="syncEpg(prof.id)">{{ t('iptv.sync_epg') }}</button>
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-danger"
            :aria-label="`Delete IPTV profile ${prof.name}`"
            @click="removeProfile(prof.id)"
          >
            <Trash2 :size="14" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>

    <div class="controls-bar">
      <input
        v-model="searchQuery"
        type="search"
        name="iptvChannelSearch"
        autocomplete="off"
        aria-label="Search IPTV channels"
        class="v-input"
        :placeholder="t('iptv.search_channels')"
        @input="runChannelSearch"
      />
      <select
        v-model="selectedGroup"
        aria-label="Channel group filter"
        class="v-select group-select"
        @change="runChannelSearch"
      >
        <option value="">{{ t('iptv.all_groups') }}</option>
        <option v-for="g in groups" :key="g" :value="g">{{ g }}</option>
      </select>
      <label class="v-switch">
        <input
          v-model="favoritesOnly"
          type="checkbox"
          class="v-switch-input"
          @change="runChannelSearch"
        />
        <span class="v-switch-track" aria-hidden="true" />
        <span>{{ t('iptv.favorites_only') }}</span>
      </label>
    </div>

    <div v-if="selectedChannel && epg" class="v-card epg-banner">
      <strong>{{ selectedChannel.name }}</strong>
      <div class="v-muted">
        <span>{{ t('iptv.now') }}: {{ epg.now?.title ?? '—' }}</span> ·
        <span>{{ t('iptv.next') }}: {{ epg.next?.title ?? '—' }}</span>
      </div>
    </div>

    <div v-if="!channels.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <div v-else class="channels-grid">
      <div
        v-for="ch in channels"
        :key="ch.id"
        class="v-card channel-card"
      >
        <div class="ch-logo-box">
          <img
            v-if="ch.logoUrl && !failedLogos[ch.id]"
            :src="ch.logoUrl"
            :alt="ch.name"
            loading="lazy"
            decoding="async"
            class="ch-logo-img"
            @error="failedLogos = { ...failedLogos, [ch.id]: true }"
          />
          <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
        </div>
        <button type="button" class="ch-info-btn" @click="selectChannel(ch)">
          <strong class="ch-name">{{ ch.name }}</strong>
          <span class="v-muted ch-group">{{ ch.groupTitle }}</span>
        </button>
        <div class="ch-actions">
          <button
            type="button"
            class="v-btn v-btn-icon"
            :aria-label="ch.isFavorite ? `Unfavorite ${ch.name}` : `Favorite ${ch.name}`"
            @click="toggleFavorite(ch)"
          >
            <Star :size="14" :fill="ch.isFavorite ? 'currentColor' : 'none'" aria-hidden="true" />
          </button>
          <button
            type="button"
            class="v-btn v-btn-icon v-btn-primary"
            :aria-label="`Play ${ch.name}`"
            @click="playChannel(ch)"
          >
            <Play :size="14" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
    </template>

    <template v-else>
      <div v-if="!livePlugins.length" class="v-card v-muted">
        {{ t('iptv.no_live_plugins') }}
      </div>

      <template v-else>
        <div class="plugin-repos-stack">
          <div v-for="group in groupedLivePlugins" :key="group.name" class="v-card live-repo-card">
            <div class="live-repo-head">
              <Package :size="16" class="repo-icon" aria-hidden="true" />
              <strong>{{ group.name }}</strong>
              <span class="v-badge">{{ group.items.length }}</span>
            </div>
            <div class="live-plugin-pills">
              <button
                v-for="plug in group.items"
                :key="plug.manifest.id"
                type="button"
                class="v-btn v-btn-sm"
                :class="{ 'v-btn-primary': selectedLivePluginId === plug.manifest.id }"
                @click="selectLivePlugin(plug.manifest.id)"
              >
                <Radio :size="13" aria-hidden="true" />
                <span>{{ plug.manifest.name }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="controls-bar">
          <input
            v-model="pluginChannelFilter"
            type="search"
            aria-label="Filter plugin channels"
            class="v-input"
            :placeholder="t('iptv.filter_channels')"
          />
          <button
            type="button"
            class="v-btn"
            :disabled="loadingPluginLive"
            @click="selectLivePlugin(selectedLivePluginId)"
          >
            <RefreshCw :size="14" :class="{ spin: loadingPluginLive }" aria-hidden="true" />
            <span>{{ t('common.refresh') }}</span>
          </button>
        </div>

        <div v-if="loadingPluginLive" class="channels-grid">
          <div v-for="n in 8" :key="n" class="v-skeleton live-skeleton" />
        </div>

        <div v-else-if="!filteredPluginLiveSections.length" class="v-card v-muted">
          {{ t('common.empty') }}
        </div>

        <div v-else class="plugin-sections-stack">
          <section v-for="sec in filteredPluginLiveSections" :key="sec.id" class="live-section">
            <div class="live-section-head">
              <h2 class="live-section-title">{{ sec.title }}</h2>
              <span class="v-badge">{{ sec.items.length }}</span>
            </div>
            <div class="channels-grid">
              <div
                v-for="item in sec.items"
                :key="item.nativeId"
                class="v-card channel-card"
              >
                <div class="ch-logo-box">
                  <img
                    v-if="item.poster && !failedPluginLogos[item.nativeId]"
                    :src="item.poster"
                    :alt="item.title"
                    loading="lazy"
                    decoding="async"
                    class="ch-logo-img"
                    @error="failedPluginLogos = { ...failedPluginLogos, [item.nativeId]: true }"
                  />
                  <Tv v-else :size="18" class="ch-logo-fallback" aria-hidden="true" />
                </div>
                <button
                  type="button"
                  class="ch-info-btn"
                  @click="playPluginLiveItem(item, sec.title)"
                >
                  <strong class="ch-name">{{ item.title }}</strong>
                  <span class="v-muted ch-group">{{ sec.title }}</span>
                </button>
                <div class="ch-actions">
                  <button
                    type="button"
                    class="v-btn v-btn-icon v-btn-primary"
                    :disabled="launchingChannelId === item.nativeId"
                    :aria-label="`Play ${item.title}`"
                    @click="playPluginLiveItem(item, sec.title)"
                  >
                    <RefreshCw v-if="launchingChannelId === item.nativeId" :size="14" class="spin" aria-hidden="true" />
                    <Play v-else :size="14" aria-hidden="true" />
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </template>
    </template>
  </section>
</template>

<style scoped>
.iptv-view {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.profiles-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem;
}

.profile-pill {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.profile-pill.is-selected {
  border-color: var(--vessel-accent);
}

.profile-select {
  background: transparent;
  border: none;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.profile-actions,
.controls-bar,
.ch-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.controls-bar .v-input {
  flex: 1;
  min-width: 200px;
}

.group-select {
  width: 200px;
}

.fav-toggle {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.channels-grid {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
}

.channel-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
}

.ch-logo-box {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  overflow: hidden;
  padding: 4px;
  flex-shrink: 0;
}

.ch-logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.ch-logo-fallback {
  color: var(--vessel-text-muted);
}

.ch-info-btn {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
  flex: 1;
  border: none;
  background: transparent;
  padding: 0;
  text-align: left;
  cursor: pointer;
}

.ch-name,
.ch-group {
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ch-name {
  font-size: 0.92rem;
}

.spin {
  animation: iptv-spin 0.9s linear infinite;
}

@keyframes iptv-spin {
  to {
    transform: rotate(360deg);
  }
}

.epg-banner {
  border-color: var(--vessel-accent-secondary);
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.iptv-tabs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.plugin-repos-stack,
.plugin-sections-stack {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.live-repo-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.live-repo-head,
.live-plugin-pills,
.live-section-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  flex-wrap: wrap;
}

.repo-icon {
  color: var(--vessel-accent);
}

.live-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.live-section-title {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 700;
}

.live-skeleton {
  height: 70px;
}
</style>
