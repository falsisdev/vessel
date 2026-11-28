<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import {
  Cloud,
  ExternalLink,
  KeyRound,
  Link2Off,
  Loader2,
  RefreshCw,
  Send,
  Sparkles,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { lastOAuthEvent, pushToast } from '@/composables/useVesselEvents'
import type {
  CloudSyncStatus,
  OAuthDevicePrompt,
  OAuthProviderKind,
  TrackerAccountStatus,
  TrackerProviderKind,
} from '@/types/vessel'

const { t } = useI18n()

const loading = shallowRef(false)
const syncingDrive = shallowRef(false)
const flushingQueue = shallowRef(false)
const busyProvider = shallowRef<string | null>(null)
const cloudStatus = shallowRef<CloudSyncStatus | null>(null)
const trackers = shallowRef<TrackerAccountStatus[]>([])
const showClientConfig = ref<Record<string, boolean>>({})
const clientIds = ref<Record<string, string>>({})
const clientSecrets = ref<Record<string, string>>({})
const devicePrompt = shallowRef<OAuthDevicePrompt | null>(null)

let devicePollTimer: ReturnType<typeof setInterval> | null = null

const totalPendingQueue = computed(() =>
  trackers.value.reduce((sum, item) => sum + (item.pendingQueueCount || 0), 0),
)

async function refreshAll(): Promise<void> {
  if (!isTauriRuntime()) return
  loading.value = true
  try {
    const [cloud, list] = await Promise.all([
      vesselApi.getCloudSyncStatus(),
      vesselApi.listTrackerAccounts(),
    ])
    cloudStatus.value = cloud
    trackers.value = list
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

function toggleClientConfig(provider: OAuthProviderKind): void {
  showClientConfig.value = {
    ...showClientConfig.value,
    [provider]: !showClientConfig.value[provider],
  }
}

async function saveClientConfig(provider: OAuthProviderKind): Promise<void> {
  const clientId = (clientIds.value[provider] ?? '').trim()
  const clientSecret = (clientSecrets.value[provider] ?? '').trim()
  if (!clientId) {
    pushToast('error', t('sync.client_id_required'))
    return
  }
  busyProvider.value = provider
  try {
    await vesselApi.saveOAuthClientConfig(
      provider,
      clientId,
      clientSecret.length > 0 ? clientSecret : undefined,
    )
    clientIds.value[provider] = ''
    clientSecrets.value[provider] = ''
    showClientConfig.value[provider] = false
    pushToast('info', t('sync.client_config_saved'))
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyProvider.value = null
  }
}

async function startBrowserOAuth(provider: OAuthProviderKind): Promise<void> {
  busyProvider.value = provider
  try {
    const res = await vesselApi.startOAuthLoopback(provider)
    if (res.kind === 'browser_url') {
      await vesselApi.openExternalUrl(res.authUrl)
      pushToast('info', t('sync.browser_auth_opened'))
    } else if (res.kind === 'device_prompt') {
      beginDevicePromptPolling(res)
    }
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyProvider.value = null
  }
}

function stopDevicePolling(): void {
  if (devicePollTimer) {
    clearInterval(devicePollTimer)
    devicePollTimer = null
  }
  devicePrompt.value = null
}

function beginDevicePromptPolling(prompt: OAuthDevicePrompt): void {
  stopDevicePolling()
  devicePrompt.value = prompt
  const intervalMs = Math.max(3000, (prompt.intervalSecs || 5) * 1000)
  devicePollTimer = setInterval(async () => {
    const active = devicePrompt.value
    if (!active) {
      stopDevicePolling()
      return
    }
    try {
      const done = await vesselApi.pollOAuthDeviceCode(active.provider, active.deviceCode)
      if (done) {
        stopDevicePolling()
        pushToast('info', t('sync.oauth_success'))
        await refreshAll()
      }
    } catch (err) {
      const msg = String(err)
      if (!msg.toLowerCase().includes('pending') && !msg.toLowerCase().includes('slow_down')) {
        stopDevicePolling()
        pushToast('error', msg)
      }
    }
  }, intervalMs)
}

async function startDeviceOAuth(provider: OAuthProviderKind): Promise<void> {
  busyProvider.value = provider
  try {
    const prompt = await vesselApi.startOAuthDeviceCode(provider)
    beginDevicePromptPolling(prompt)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyProvider.value = null
  }
}

async function toggleCloudAutoSync(): Promise<void> {
  if (!cloudStatus.value) return
  const next = !cloudStatus.value.autoSync
  try {
    await vesselApi.setCloudSyncAuto(next)
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleSyncDriveNow(): Promise<void> {
  syncingDrive.value = true
  try {
    const res = await vesselApi.syncGoogleDriveNow()
    cloudStatus.value = res.status
    const total = res.merge.libraryApplied + res.merge.progressApplied + res.merge.overridesApplied
    pushToast('info', t('sync.drive_synced', { count: total }))
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    syncingDrive.value = false
  }
}

async function handleDisconnectDrive(): Promise<void> {
  busyProvider.value = 'google_drive'
  try {
    await vesselApi.disconnectGoogleDrive()
    pushToast('info', t('sync.disconnected'))
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyProvider.value = null
  }
}

async function handleDisconnectTracker(provider: TrackerProviderKind): Promise<void> {
  busyProvider.value = provider
  try {
    await vesselApi.disconnectTracker(provider)
    pushToast('info', t('sync.disconnected'))
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyProvider.value = null
  }
}

async function toggleTrackerPref(
  acc: TrackerAccountStatus,
  field: 'autoScrobble' | 'syncLibrary',
): Promise<void> {
  const autoScrobble = field === 'autoScrobble' ? !acc.autoScrobble : acc.autoScrobble
  const syncLibrary = field === 'syncLibrary' ? !acc.syncLibrary : acc.syncLibrary
  try {
    await vesselApi.setTrackerPreferences(acc.provider, autoScrobble, syncLibrary)
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleFlushQueue(): Promise<void> {
  flushingQueue.value = true
  try {
    const sent = await vesselApi.flushTrackerQueue()
    pushToast('info', t('sync.queue_flushed', { count: sent }))
    await refreshAll()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    flushingQueue.value = false
  }
}

watch(lastOAuthEvent, (evt) => {
  if (!evt) return
  if (evt.success) {
    pushToast('info', t('sync.oauth_success'))
  } else if (evt.error) {
    pushToast('error', evt.error)
  }
  void refreshAll()
})

onMounted(() => {
  void refreshAll()
})

onBeforeUnmount(() => {
  stopDevicePolling()
})
</script>

<template>
  <div class="v-card section-card sync-section">
    <div class="section-title-row">
      <Cloud :size="18" class="section-icon" aria-hidden="true" />
      <h3>{{ t('sync.drive_title') }}</h3>
    </div>
    <p class="v-muted sync-sub">{{ t('sync.drive_subtitle') }}</p>

    <div v-if="loading && !cloudStatus" class="v-muted inline-row">
      <Loader2 :size="15" class="spin" /> Loading...
    </div>

    <template v-else-if="cloudStatus">
      <div class="sync-status-card">
        <div class="sync-card-head">
          <div class="sync-meta-row">
            <strong>Google Drive (appDataFolder)</strong>
            <span class="v-badge" :class="cloudStatus.connected ? 'v-badge-success' : ''">
              {{
                cloudStatus.connected
                  ? cloudStatus.accountEmail || t('debrid.connected')
                  : t('debrid.not_configured')
              }}
            </span>
            <span v-if="cloudStatus.hasClientConfig" class="v-badge v-badge-accent">OAuth Client</span>
            <span class="v-badge">
              {{ t('sync.tombstones_count', { count: cloudStatus.activeTombstones }) }}
            </span>
          </div>
          <div class="inline-row">
            <button
              type="button"
              class="v-btn v-btn-sm"
              @click="toggleClientConfig('google_drive')"
            >
              <KeyRound :size="13" /> {{ t('sync.configure_oauth') }}
            </button>
            <button
              type="button"
              class="v-btn v-btn-sm v-btn-primary"
              :disabled="busyProvider === 'google_drive'"
              @click="startBrowserOAuth('google_drive')"
            >
              <ExternalLink :size="13" />
              {{ cloudStatus.connected ? t('sync.reconnect') : t('sync.connect_google_drive') }}
            </button>
            <button
              v-if="cloudStatus.connected"
              type="button"
              class="v-btn v-btn-sm"
              :disabled="syncingDrive"
              @click="handleSyncDriveNow"
            >
              <RefreshCw :size="13" :class="{ spin: syncingDrive }" /> {{ t('sync.sync_now') }}
            </button>
            <button
              v-if="cloudStatus.connected"
              type="button"
              class="v-btn v-btn-sm v-btn-danger"
              :disabled="busyProvider === 'google_drive'"
              @click="handleDisconnectDrive"
            >
              <Link2Off :size="13" /> {{ t('debrid.disconnect') }}
            </button>
          </div>
        </div>

        <div class="v-muted sync-details">
          <span>Device: <code>{{ cloudStatus.deviceId || '—' }}</code></span>
          <span v-if="cloudStatus.lastSyncedAt">
            · {{ t('sync.last_synced') }}: {{ cloudStatus.lastSyncedAt }}
          </span>
          <span v-if="cloudStatus.lastError" class="sync-err">· {{ cloudStatus.lastError }}</span>
        </div>

        <div v-if="showClientConfig['google_drive']" class="client-config-box">
          <input
            v-model="clientIds['google_drive']"
            type="text"
            autocomplete="off"
            class="v-input"
            :placeholder="t('sync.client_id_placeholder')"
          />
          <input
            v-model="clientSecrets['google_drive']"
            type="password"
            autocomplete="off"
            class="v-input"
            :placeholder="t('sync.client_secret_placeholder')"
          />
          <button
            type="button"
            class="v-btn v-btn-sm v-btn-primary"
            :disabled="busyProvider === 'google_drive'"
            @click="saveClientConfig('google_drive')"
          >
            {{ t('common.save') }}
          </button>
        </div>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('sync.auto_sync') }}</strong>
          <span class="v-muted">{{ t('sync.auto_sync_desc') }}</span>
        </div>
        <label class="v-switch">
          <input
            :checked="cloudStatus.autoSync"
            type="checkbox"
            class="v-switch-input"
            @change="toggleCloudAutoSync"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>
    </template>
  </div>

  <div class="v-card section-card sync-section">
    <div class="section-title-row">
      <Sparkles :size="18" class="section-icon" aria-hidden="true" />
      <h3>{{ t('sync.trackers_title') }}</h3>
      <button
        type="button"
        class="v-btn v-btn-sm"
        style="margin-left: auto"
        :disabled="flushingQueue"
        @click="handleFlushQueue"
      >
        <Send :size="13" :class="{ spin: flushingQueue }" />
        {{ t('sync.flush_queue') }}
        <span v-if="totalPendingQueue > 0" class="v-badge v-badge-accent">{{ totalPendingQueue }}</span>
      </button>
    </div>
    <p class="v-muted sync-sub">{{ t('sync.trackers_subtitle') }}</p>

    <div v-if="devicePrompt" class="oauth-device-box">
      <span>{{ t('debrid.device_auth_prompt', { url: devicePrompt.verificationUri }) }}</span>
      <strong class="oauth-user-code">{{ devicePrompt.userCode }}</strong>
      <div class="inline-row">
        <button
          type="button"
          class="v-btn v-btn-sm v-btn-primary"
          @click="
            vesselApi.openExternalUrl(
              devicePrompt.verificationUriComplete || devicePrompt.verificationUri,
            )
          "
        >
          <ExternalLink :size="13" /> {{ devicePrompt.verificationUri }}
        </button>
        <span class="v-muted"><Loader2 :size="13" class="spin" /> {{ t('debrid.device_auth_waiting') }}</span>
        <button type="button" class="v-btn v-btn-sm" @click="stopDevicePolling">
          {{ t('common.cancel') }}
        </button>
      </div>
    </div>

    <div class="tracker-list">
      <div v-for="acc in trackers" :key="acc.provider" class="sync-status-card">
        <div class="sync-card-head">
          <div class="sync-meta-row">
            <strong>{{ acc.displayName }}</strong>
            <span class="v-badge" :class="acc.connected ? 'v-badge-success' : ''">
              {{
                acc.connected
                  ? acc.username || t('debrid.connected')
                  : acc.implemented
                    ? t('debrid.not_configured')
                    : t('sync.coming_soon_slot')
              }}
            </span>
            <span v-if="!acc.implemented" class="v-badge">Phase 2B</span>
            <span v-if="acc.pendingQueueCount > 0" class="v-badge v-badge-accent">
              {{ t('sync.pending_count', { count: acc.pendingQueueCount }) }}
            </span>
          </div>

          <div class="inline-row">
            <button
              type="button"
              class="v-btn v-btn-sm"
              @click="toggleClientConfig(acc.provider)"
            >
              <KeyRound :size="13" /> {{ t('sync.configure_oauth') }}
            </button>

            <template v-if="acc.implemented">
              <button
                v-if="acc.provider === 'trakt'"
                type="button"
                class="v-btn v-btn-sm"
                :disabled="busyProvider === acc.provider"
                @click="startDeviceOAuth(acc.provider)"
              >
                <ExternalLink :size="13" /> {{ t('debrid.connect_device') }}
              </button>
              <button
                type="button"
                class="v-btn v-btn-sm v-btn-primary"
                :disabled="busyProvider === acc.provider"
                @click="startBrowserOAuth(acc.provider)"
              >
                <ExternalLink :size="13" />
                {{ acc.connected ? t('sync.reconnect') : t('sync.connect_oauth') }}
              </button>
              <button
                v-if="acc.connected"
                type="button"
                class="v-btn v-btn-sm v-btn-danger"
                :disabled="busyProvider === acc.provider"
                @click="handleDisconnectTracker(acc.provider)"
              >
                <Link2Off :size="13" /> {{ t('debrid.disconnect') }}
              </button>
            </template>
          </div>
        </div>

        <div v-if="showClientConfig[acc.provider]" class="client-config-box">
          <input
            v-model="clientIds[acc.provider]"
            type="text"
            autocomplete="off"
            class="v-input"
            :placeholder="t('sync.client_id_placeholder')"
          />
          <input
            v-model="clientSecrets[acc.provider]"
            type="password"
            autocomplete="off"
            class="v-input"
            :placeholder="t('sync.client_secret_placeholder')"
          />
          <button
            type="button"
            class="v-btn v-btn-sm v-btn-primary"
            :disabled="busyProvider === acc.provider"
            @click="saveClientConfig(acc.provider)"
          >
            {{ t('common.save') }}
          </button>
        </div>

        <div v-if="acc.implemented" class="tracker-toggles">
          <label class="tracker-toggle-chip">
            <input
              :checked="acc.autoScrobble"
              type="checkbox"
              @change="toggleTrackerPref(acc, 'autoScrobble')"
            />
            <span>{{ t('sync.auto_scrobble') }}</span>
          </label>
          <label class="tracker-toggle-chip">
            <input
              :checked="acc.syncLibrary"
              type="checkbox"
              @change="toggleTrackerPref(acc, 'syncLibrary')"
            />
            <span>{{ t('sync.sync_library') }}</span>
          </label>
          <span v-if="acc.lastSyncedAt" class="v-muted sync-details">
            {{ t('sync.last_synced') }}: {{ acc.lastSyncedAt }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sync-sub {
  margin: -0.25rem 0 0.5rem;
  font-size: 0.84rem;
}
.sync-status-card {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  padding: 0.75rem 0.9rem;
  border-radius: var(--vessel-radius);
  background: var(--vessel-bg-elevated);
  border: 1px solid var(--vessel-border);
}
.sync-card-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
}
.sync-meta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
}
.sync-details {
  font-size: 0.78rem;
}
.sync-err {
  color: var(--vessel-danger);
}
.client-config-box {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding-top: 0.35rem;
}
.client-config-box .v-input {
  flex: 1 1 200px;
}
.tracker-list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.tracker-toggles {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 1rem;
  font-size: 0.82rem;
}
.tracker-toggle-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  cursor: pointer;
}
.oauth-device-box {
  padding: 0.65rem 0.75rem;
  border-radius: var(--vessel-radius);
  background: var(--vessel-bg-surface);
  border: 1px dashed var(--vessel-accent);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.oauth-user-code {
  font-size: 1.2rem;
  letter-spacing: 0.12em;
  color: var(--vessel-accent);
}
</style>
