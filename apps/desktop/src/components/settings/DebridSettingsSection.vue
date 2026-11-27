<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import {
  ArrowDown,
  ArrowUp,
  Cloud,
  ExternalLink,
  HardDriveDownload,
  KeyRound,
  Loader2,
  Play,
  RefreshCw,
  Trash2,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { formatBytes } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'
import { pushToast } from '@/composables/useVesselEvents'
import type {
  DebridAccountStatus,
  DebridCloudItem,
  DebridPreferences,
  RealDebridDeviceAuthStart,
} from '@/types/vessel'

const { t } = useI18n()

const loading = shallowRef(true)
const accounts = ref<DebridAccountStatus[]>([])
const prefs = ref<DebridPreferences>({
  priorityOrder: [],
  onlyCached: false,
  autoFallback: true,
})
const apiKeyInputs = ref<Record<string, string>>({})
const busyTarget = shallowRef('')

const rdAuth = ref<RealDebridDeviceAuthStart | null>(null)
const rdPolling = shallowRef(false)
let rdPollTimer: ReturnType<typeof setInterval> | null = null

const cloudItems = ref<DebridCloudItem[]>([])
const loadingCloud = shallowRef(false)
const cloudAutoPoll = shallowRef(false)
let cloudPollTimer: ReturnType<typeof setInterval> | null = null

const orderedAccounts = computed(() => {
  const map = new Map<string, DebridAccountStatus>()
  for (const acc of accounts.value) map.set(acc.target, acc)
  const list: DebridAccountStatus[] = []
  for (const target of prefs.value.priorityOrder) {
    const found = map.get(target)
    if (found) {
      list.push(found)
      map.delete(target)
    }
  }
  for (const rest of map.values()) list.push(rest)
  return list
})

async function refreshOverview(): Promise<void> {
  if (!isTauriRuntime()) {
    loading.value = false
    return
  }
  loading.value = true
  try {
    const ov = await vesselApi.getDebridOverview()
    accounts.value = ov.accounts
    prefs.value = ov.preferences
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loading.value = false
  }
}

async function refreshCloud(silent = false): Promise<void> {
  if (!isTauriRuntime()) return
  if (!silent) loadingCloud.value = true
  try {
    cloudItems.value = await vesselApi.listDebridCloud()
  } catch (err) {
    if (!silent) pushToast('error', String(err))
  } finally {
    loadingCloud.value = false
  }
}

function toggleCloudAutoPoll(): void {
  cloudAutoPoll.value = !cloudAutoPoll.value
  if (cloudPollTimer) {
    clearInterval(cloudPollTimer)
    cloudPollTimer = null
  }
  if (cloudAutoPoll.value) {
    void refreshCloud(true)
    cloudPollTimer = setInterval(() => {
      void refreshCloud(true)
    }, 2000)
  }
}

async function saveKey(target: string): Promise<void> {
  const val = (apiKeyInputs.value[target] ?? '').trim()
  if (!val) return
  busyTarget.value = target
  try {
    await vesselApi.saveDebridApiKey(target, val)
    apiKeyInputs.value[target] = ''
    await refreshOverview()
    pushToast('info', `${target}: ${t('debrid.connected')}`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyTarget.value = ''
  }
}

async function disconnectProvider(target: string): Promise<void> {
  busyTarget.value = target
  try {
    await vesselApi.removeDebridCredentials(target)
    await refreshOverview()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    busyTarget.value = ''
  }
}

function stopRdPolling(): void {
  if (rdPollTimer) {
    clearInterval(rdPollTimer)
    rdPollTimer = null
  }
  rdPolling.value = false
}

async function startRdDeviceAuth(): Promise<void> {
  stopRdPolling()
  try {
    const started = await vesselApi.startRealDebridDeviceAuth()
    rdAuth.value = started
    rdPolling.value = true
    const intervalMs = Math.max(3, started.intervalSeconds || 5) * 1000
    rdPollTimer = setInterval(async () => {
      if (!rdAuth.value) {
        stopRdPolling()
        return
      }
      try {
        const poll = await vesselApi.pollRealDebridDeviceAuth(rdAuth.value.deviceCode)
        if (poll.authorized) {
          stopRdPolling()
          rdAuth.value = null
          await refreshOverview()
          pushToast('info', `Real-Debrid: ${poll.username || t('debrid.connected')}`)
        }
      } catch (err) {
        stopRdPolling()
        pushToast('error', String(err))
      }
    }, intervalMs)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function movePriority(idx: number, delta: -1 | 1): Promise<void> {
  const order = orderedAccounts.value.map((a) => a.target)
  const nextIdx = idx + delta
  if (nextIdx < 0 || nextIdx >= order.length) return
  const [item] = order.splice(idx, 1)
  order.splice(nextIdx, 0, item)
  await savePrefs({ ...prefs.value, priorityOrder: order })
}

async function savePrefs(next: DebridPreferences): Promise<void> {
  prefs.value = next
  try {
    prefs.value = await vesselApi.setDebridPreferences(next)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function removeCloudItem(item: DebridCloudItem): Promise<void> {
  try {
    await vesselApi.deleteDebridCloud(item.target, item.id)
    cloudItems.value = cloudItems.value.filter((x) => !(x.target === item.target && x.id === item.id))
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function queueCloudDownload(item: DebridCloudItem): Promise<void> {
  const rawLink = item.links[0]
  if (!rawLink) return
  try {
    const resolved = await vesselApi.unrestrictDebridCloudLink(item.target, rawLink)
    await vesselApi.enqueueDownload({
      pluginId: 'debrid',
      nativeId: item.infoHash || item.id,
      title: resolved.filename || item.filename,
      streamLabel: item.providerLabel,
      kind: 'direct',
      sourceUrl: resolved.url,
      headers: resolved.headers ?? {},
    })
    pushToast('info', `Queued download: ${resolved.filename || item.filename}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

function formatSpeed(bps?: number): string {
  if (!bps || bps <= 0) return '—'
  return `${formatBytes(bps)}/s`
}

function formatEta(secs?: number): string {
  if (!secs || secs <= 0) return ''
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

onMounted(() => {
  void refreshOverview()
  void refreshCloud(true)
})

onBeforeUnmount(() => {
  stopRdPolling()
  if (cloudPollTimer) clearInterval(cloudPollTimer)
})
</script>

<template>
  <div class="v-card section-card debrid-section">
    <div class="section-title-row">
      <Cloud :size="18" class="section-icon" aria-hidden="true" />
      <h3>{{ t('debrid.title') }}</h3>
    </div>
    <p class="v-muted debrid-sub">{{ t('debrid.subtitle') }} {{ t('debrid.priority_hint') }}</p>

    <div class="v-setting-row">
      <div class="setting-copy">
        <strong>{{ t('debrid.only_cached') }}</strong>
        <span class="v-muted">{{ t('debrid.only_cached_desc') }}</span>
      </div>
      <label class="v-switch">
        <input
          :checked="prefs.onlyCached"
          type="checkbox"
          class="v-switch-input"
          @change="savePrefs({ ...prefs, onlyCached: !prefs.onlyCached })"
        />
        <span class="v-switch-track" aria-hidden="true" />
      </label>
    </div>

    <div class="v-setting-row">
      <div class="setting-copy">
        <strong>{{ t('debrid.auto_fallback') }}</strong>
        <span class="v-muted">{{ t('debrid.auto_fallback_desc') }}</span>
      </div>
      <label class="v-switch">
        <input
          :checked="prefs.autoFallback"
          type="checkbox"
          class="v-switch-input"
          @change="savePrefs({ ...prefs, autoFallback: !prefs.autoFallback })"
        />
        <span class="v-switch-track" aria-hidden="true" />
      </label>
    </div>

    <div v-if="loading" class="v-muted inline-row">
      <Loader2 :size="15" class="spin" /> Loading...
    </div>

    <div v-else class="debrid-provider-list">
      <div v-for="(acc, idx) in orderedAccounts" :key="acc.target" class="debrid-provider-card">
        <div class="debrid-provider-head">
          <div class="debrid-provider-meta">
            <strong>{{ acc.label }}</strong>
            <span v-if="!acc.isBuiltin" class="v-badge">Plugin</span>
            <span
              class="v-badge"
              :class="acc.authenticated ? 'v-badge-success' : acc.error ? 'v-badge-danger' : ''"
            >
              {{ acc.authenticated ? (acc.username || t('debrid.connected')) : t('debrid.not_configured') }}
            </span>
            <span v-if="acc.authenticated" class="v-badge v-badge-accent">
              {{ acc.isPremium ? t('debrid.premium') : t('debrid.free') }}
            </span>
            <span v-if="acc.pointsOrQuota" class="v-muted debrid-meta-sm">{{ acc.pointsOrQuota }}</span>
            <span v-if="acc.expiresAt" class="v-muted debrid-meta-sm">{{ acc.expiresAt }}</span>
          </div>
          <div class="debrid-order-btns">
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-sm"
              :disabled="idx === 0"
              aria-label="Move up"
              @click="movePriority(idx, -1)"
            >
              <ArrowUp :size="13" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-icon v-btn-sm"
              :disabled="idx === orderedAccounts.length - 1"
              aria-label="Move down"
              @click="movePriority(idx, 1)"
            >
              <ArrowDown :size="13" />
            </button>
          </div>
        </div>

        <p v-if="acc.error" class="debrid-err-text">{{ acc.error }}</p>

        <div v-if="acc.isBuiltin" class="inline-row">
          <input
            v-model="apiKeyInputs[acc.target]"
            type="password"
            autocomplete="off"
            class="v-input"
            :placeholder="t('debrid.api_key_placeholder')"
          />
          <button
            type="button"
            class="v-btn v-btn-sm v-btn-primary"
            :disabled="busyTarget === acc.target"
            @click="saveKey(acc.target)"
          >
            <KeyRound :size="13" /> {{ t('debrid.save_key') }}
          </button>
          <button
            v-if="acc.target === 'builtin:real_debrid'"
            type="button"
            class="v-btn v-btn-sm"
            @click="startRdDeviceAuth"
          >
            <ExternalLink :size="13" /> {{ t('debrid.connect_device') }}
          </button>
          <button
            v-if="acc.enabled"
            type="button"
            class="v-btn v-btn-sm v-btn-danger"
            :disabled="busyTarget === acc.target"
            @click="disconnectProvider(acc.target)"
          >
            {{ t('debrid.disconnect') }}
          </button>
        </div>

        <div v-if="acc.target === 'builtin:real_debrid' && rdAuth" class="rd-device-box">
          <span>{{ t('debrid.device_auth_prompt', { url: rdAuth.verificationUrl }) }}</span>
          <strong class="rd-user-code">{{ rdAuth.userCode }}</strong>
          <div class="inline-row">
            <button
              type="button"
              class="v-btn v-btn-sm v-btn-primary"
              @click="vesselApi.openExternalUrl(rdAuth.directVerificationUrl || rdAuth.verificationUrl)"
            >
              <ExternalLink :size="13" /> {{ rdAuth.verificationUrl }}
            </button>
            <span class="v-muted"><Loader2 :size="13" class="spin" /> {{ t('debrid.device_auth_waiting') }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="debrid-cloud-box">
      <div class="debrid-cloud-head">
        <strong>{{ t('debrid.cloud_title') }}</strong>
        <div class="debrid-cloud-actions">
          <label class="v-switch">
            <input
              :checked="cloudAutoPoll"
              type="checkbox"
              class="v-switch-input"
              @change="toggleCloudAutoPoll"
            />
            <span class="v-switch-track" aria-hidden="true" />
            <span class="v-muted debrid-meta-sm">{{ t('debrid.cloud_auto_poll') }}</span>
          </label>
          <button type="button" class="v-btn v-btn-sm" :disabled="loadingCloud" @click="refreshCloud(false)">
            <RefreshCw :size="13" :class="{ spin: loadingCloud }" /> {{ t('debrid.cloud_refresh') }}
          </button>
        </div>
      </div>

      <p v-if="!cloudItems.length" class="v-muted debrid-meta-sm">{{ t('debrid.cloud_empty') }}</p>

      <div v-else class="debrid-cloud-list">
        <div v-for="item in cloudItems" :key="`${item.target}:${item.id}`" class="debrid-cloud-row">
          <div class="debrid-cloud-info">
            <strong class="debrid-cloud-name">{{ item.filename }}</strong>
            <div class="debrid-cloud-meta">
              <span class="v-badge v-badge-accent">{{ item.providerLabel }}</span>
              <span class="v-badge">{{ item.status.toUpperCase() }} {{ Math.round(item.progressPct) }}%</span>
              <span v-if="item.sizeBytes" class="v-muted">{{ formatBytes(item.sizeBytes) }}</span>
              <span v-if="item.speedBps" class="v-muted">{{ formatSpeed(item.speedBps) }}</span>
              <span v-if="item.seeders !== undefined" class="v-muted">👤 {{ item.seeders }}</span>
              <span v-if="item.etaSeconds" class="v-muted">ETA {{ formatEta(item.etaSeconds) }}</span>
            </div>
            <div class="debrid-progress-track">
              <div class="debrid-progress-fill" :style="{ width: `${Math.min(100, Math.max(0, item.progressPct))}%` }" />
            </div>
          </div>
          <div class="debrid-order-btns">
            <button
              v-if="item.status === 'ready' && item.links.length"
              type="button"
              class="v-btn v-btn-sm v-btn-primary"
              title="Download"
              @click="queueCloudDownload(item)"
            >
              <HardDriveDownload :size="13" />
            </button>
            <button
              type="button"
              class="v-btn v-btn-sm v-btn-danger"
              title="Delete"
              @click="removeCloudItem(item)"
            >
              <Trash2 :size="13" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.debrid-section {
  width: 100%;
}
.debrid-sub {
  margin: 0;
  font-size: 0.85rem;
}
.debrid-provider-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  width: 100%;
}
.debrid-provider-card {
  width: 100%;
  padding: 0.85rem;
  border-radius: var(--vessel-radius);
  border: 1px solid var(--vessel-border);
  background: var(--vessel-bg-elevated);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.debrid-provider-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}
.debrid-provider-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
}
.debrid-meta-sm {
  font-size: 0.78rem;
}
.debrid-order-btns {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}
.debrid-err-text {
  margin: 0;
  color: var(--vessel-danger);
  font-size: 0.8rem;
}
.rd-device-box {
  padding: 0.65rem 0.75rem;
  border-radius: var(--vessel-radius);
  background: var(--vessel-bg-surface);
  border: 1px dashed var(--vessel-accent);
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.rd-user-code {
  font-size: 1.2rem;
  letter-spacing: 0.12em;
  color: var(--vessel-accent);
}
.debrid-cloud-box {
  width: 100%;
  margin-top: 0.5rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--vessel-border);
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}
.debrid-cloud-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.debrid-cloud-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.debrid-cloud-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}
.debrid-cloud-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.6rem 0.75rem;
  border-radius: var(--vessel-radius);
  background: var(--vessel-bg-elevated);
  border: 1px solid var(--vessel-border);
}
.debrid-cloud-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.debrid-cloud-name {
  font-size: 0.86rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.debrid-cloud-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.76rem;
}
.debrid-progress-track {
  height: 4px;
  border-radius: 999px;
  background: var(--vessel-border);
  overflow: hidden;
}
.debrid-progress-fill {
  height: 100%;
  background: var(--vessel-accent);
  transition: width 0.25s ease;
}
</style>
