<script setup lang="ts">
import { onMounted, ref, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { Play, RefreshCw, Star, Trash2, Tv } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import IptvProfileForm from '@/components/iptv/IptvProfileForm.vue'
import { useI18n } from '@/composables/useI18n'
import { usePlayerSession } from '@/composables/usePlayerSession'
import { pushToast } from '@/composables/useVesselEvents'
import type { EpgNowNext, IptvKind, StoredIptvChannel, StoredIptvProfile, StreamCandidate } from '@/types/vessel'

const router = useRouter()
const { t } = useI18n()
const { setSession } = usePlayerSession()

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
  const toCandidate = (item: StoredIptvChannel): StreamCandidate => {
    const itemProf = profiles.value.find((p) => p.id === item.profileId)
    const itemHeaders: Record<string, string> = {}
    if (itemProf?.userAgent) itemHeaders['User-Agent'] = itemProf.userAgent

    // Upgrade legacy saved Xtream `.ts` URLs to `.m3u8` for native WebKit HLS playback
    let resolvedUrl = item.streamUrl
    if (
      item.streamId.startsWith('xtream-') &&
      resolvedUrl.toLowerCase().endsWith('.ts') &&
      resolvedUrl.includes('/live/')
    ) {
      resolvedUrl = `${resolvedUrl.slice(0, -3)}.m3u8`
    }
    const isHls = resolvedUrl.toLowerCase().includes('.m3u8')
    return {
      url: resolvedUrl,
      kind: isHls ? 'hls' : 'mpeg_ts',
      label: isHls ? `${item.name} (HLS)` : item.name,
      flags: ['requires_proxy'],
      headers: itemHeaders,
      subtitles: [],
    }
  }

  const channelStreams: Record<string, StreamCandidate> = {}
  const episodes = channels.value.map((item, idx) => {
    channelStreams[item.streamId] = toCandidate(item)
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
  if (!channelStreams[ch.streamId]) {
    channelStreams[ch.streamId] = toCandidate(ch)
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
    streams: [toCandidate(ch)],
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

onMounted(() => {
  void refreshProfiles()
})
</script>

<template>
  <section class="iptv-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('iptv.title') }}</h1>
    </header>

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
  background: rgba(10, 11, 18, 0.78);
  border: 1px solid rgba(255, 255, 255, 0.08);
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
</style>
