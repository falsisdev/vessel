<script setup lang="ts">
import { onMounted, ref, shallowRef } from 'vue'
import { Download, RefreshCw, Settings2, ShieldCheck, Trash2 } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import PluginSettingsDrawer from '@/components/extensions/PluginSettingsDrawer.vue'
import { useI18n } from '@/composables/useI18n'
import { pushToast } from '@/composables/useVesselEvents'
import type { PluginListEntry, PluginManifest, ResolvedSource } from '@/types/vessel'

const { t } = useI18n()

const sourceInput = shallowRef('')
const resolving = shallowRef(false)
const resolvedPreview = ref<ResolvedSource | null>(null)
const plugins = ref<PluginListEntry[]>([])
const activeSettingsManifest = ref<PluginManifest | null>(null)
const confirmUninstallId = shallowRef<string>('')

async function refreshPlugins(): Promise<void> {
  if (!isTauriRuntime()) return
  try {
    plugins.value = await vesselApi.listPlugins()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function inspectSource(): Promise<void> {
  const raw = sourceInput.value.trim()
  if (!raw) return
  resolving.value = true
  try {
    resolvedPreview.value = await vesselApi.resolveSource(raw)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    resolving.value = false
  }
}

async function installSourceTarget(target: string): Promise<void> {
  try {
    const res = await vesselApi.installSource(target)
    resolvedPreview.value = null
    sourceInput.value = ''
    await refreshPlugins()
    pushToast('info', `Installed ${res.pluginId} (${res.badge})`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function toggleEnabled(entry: PluginListEntry): Promise<void> {
  try {
    await vesselApi.setPluginEnabled(entry.manifest.id, !entry.enabled)
    await refreshPlugins()
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function removePlugin(pluginId: string): Promise<void> {
  if (confirmUninstallId.value !== pluginId) {
    confirmUninstallId.value = pluginId
    setTimeout(() => {
      if (confirmUninstallId.value === pluginId) confirmUninstallId.value = ''
    }, 3000)
    return
  }
  confirmUninstallId.value = ''
  try {
    await vesselApi.uninstallPlugin(pluginId)
    if (activeSettingsManifest.value?.id === pluginId) {
      activeSettingsManifest.value = null
    }
    await refreshPlugins()
  } catch (err) {
    pushToast('error', String(err))
  }
}

onMounted(() => {
  void refreshPlugins()
})
</script>

<template>
  <section class="extensions-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('extensions.title') }}</h1>
      <button class="v-btn" @click="refreshPlugins">
        <RefreshCw :size="15" /> {{ t('common.refresh') }}
      </button>
    </header>

    <form class="source-bar" @submit.prevent="inspectSource">
      <input
        v-model="sourceInput"
        name="pluginSource"
        autocomplete="off"
        spellcheck="false"
        aria-label="Plugin or repository source"
        class="v-input"
        :placeholder="t('extensions.source_placeholder')"
      />
      <button type="submit" class="v-btn v-btn-primary" :disabled="resolving">
        {{ resolving ? t('common.loading') : t('extensions.resolve') }}
      </button>
    </form>

    <div v-if="resolvedPreview" class="v-card preview-card">
      <div class="preview-head">
        <strong>
          {{
            resolvedPreview.kind === 'repository'
              ? resolvedPreview.repo.name
              : resolvedPreview.manifest.name
          }}
        </strong>
        <span
          class="v-badge"
          :class="resolvedPreview.badge === 'official' ? 'v-badge-success' : 'v-badge-accent'"
        >
          <ShieldCheck :size="12" />
          {{ resolvedPreview.badge === 'official' ? t('extensions.official') : t('extensions.community') }}
        </span>
      </div>

      <template v-if="resolvedPreview.kind === 'package'">
        <p class="v-muted preview-meta">
          v{{ resolvedPreview.manifest.version }} · SHA-256:
          <code>{{ resolvedPreview.sha256.slice(0, 12) }}...</code>
        </p>
        <button class="v-btn v-btn-primary" @click="installSourceTarget(sourceInput.trim())">
          <Download :size="15" /> {{ t('extensions.install_now') }}
        </button>
      </template>

      <template v-else>
        <div class="repo-list">
          <div
            v-for="rp in resolvedPreview.repo.plugins"
            :key="rp.id"
            class="repo-plugin-row"
          >
            <div>
              <strong>{{ rp.name }}</strong>
              <span class="v-muted"> v{{ rp.version }}</span>
            </div>
            <button
              v-if="rp.downloadUrl || rp.sourcePath"
              class="v-btn v-btn-primary"
              @click="installSourceTarget((rp.downloadUrl ?? rp.sourcePath)!)"
            >
              {{ t('common.install') }}
            </button>
          </div>
        </div>
      </template>
    </div>

    <PluginSettingsDrawer
      v-if="activeSettingsManifest"
      :manifest="activeSettingsManifest"
      @close="activeSettingsManifest = null"
    />

    <div v-if="!plugins.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <div v-else class="plugin-list">
      <div v-for="entry in plugins" :key="entry.manifest.id" class="v-card plugin-row">
        <div class="plugin-info">
          <div class="plugin-head">
            <strong>{{ entry.manifest.name }}</strong>
            <span class="v-badge">v{{ entry.manifest.version }}</span>
            <span class="v-badge">{{ entry.sourceType }}</span>
            <span
              class="v-badge"
              :class="entry.enabled ? 'v-badge-success' : ''"
            >
              {{ entry.enabled ? t('common.enabled') : t('common.disabled') }}
            </span>
          </div>
          <p v-if="entry.manifest.description" class="v-muted plugin-desc">
            {{ entry.manifest.description }}
          </p>
          <div class="domain-row v-muted">
            <span>{{ t('extensions.domains') }}: {{ entry.allowedDomains.join(', ') || 'none' }}</span>
          </div>
        </div>

        <div class="plugin-actions">
          <button
            v-if="entry.sourceType === 'dev_dir'"
            class="v-btn"
            @click="installSourceTarget(entry.localPath)"
          >
            <RefreshCw :size="14" /> Reload
          </button>
          <button
            v-if="entry.manifest.settingsSchema.length"
            class="v-btn"
            @click="activeSettingsManifest = entry.manifest"
          >
            <Settings2 :size="14" /> {{ t('extensions.settings') }}
          </button>
          <button class="v-btn" @click="toggleEnabled(entry)">
            {{ entry.enabled ? t('common.disable') : t('common.enable') }}
          </button>
          <button type="button" class="v-btn v-btn-danger" @click="removePlugin(entry.manifest.id)">
            <Trash2 :size="14" aria-hidden="true" />
            {{
              confirmUninstallId === entry.manifest.id ? 'Confirm Uninstall' : t('common.uninstall')
            }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.extensions-view,
.plugin-list,
.repo-list {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.source-bar {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.source-bar .v-input {
  flex: 1;
  min-width: 240px;
}

.preview-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
  border-color: var(--vessel-accent);
}

.preview-head,
.plugin-head,
.plugin-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.preview-meta,
.plugin-desc {
  margin: 0;
}

.plugin-row,
.repo-plugin-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  width: 100%;
}

.plugin-info {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.domain-row {
  font-size: 0.82rem;
}
</style>
