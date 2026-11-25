<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from 'vue'
import {
  Check,
  ChevronDown,
  ChevronRight,
  Download,
  Globe,
  Home,
  Loader2,
  Package,
  RefreshCw,
  Settings2,
  ShieldCheck,
  Trash2,
  X,
} from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import PluginSettingsDrawer from '@/components/extensions/PluginSettingsDrawer.vue'
import { useHomeCache } from '@/composables/useHomeCache'
import { useI18n } from '@/composables/useI18n'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { PluginListEntry, PluginManifest, ResolvedSource } from '@/types/vessel'

defineOptions({ name: 'ExtensionsView' })

const { t } = useI18n()
const { uiPreferences, savePreferences } = useUiPreferences()
const { invalidateHomeCache } = useHomeCache()

const sourceInput = shallowRef('')
const resolving = shallowRef(false)
const resolvedPreview = ref<ResolvedSource | null>(null)
const repoFilter = shallowRef('')
const installedFilter = shallowRef('')
const installingTarget = shallowRef('')
const batchInstalling = shallowRef(false)
const batchProgress = shallowRef('')
const plugins = ref<PluginListEntry[]>([])
const loadingPlugins = shallowRef(isTauriRuntime())
const expandedGroups = ref<Record<string, boolean>>({})
const activeSettingsManifest = ref<PluginManifest | null>(null)
const confirmUninstallId = shallowRef<string>('')
const confirmRemoveGroup = shallowRef<string>('')
const failedImageUrls = ref<Record<string, boolean>>({})

function markImageFailed(url?: string): void {
  if (!url) return
  failedImageUrls.value = { ...failedImageUrls.value, [url]: true }
}

function canShowImage(url?: string): url is string {
  return Boolean(url && !failedImageUrls.value[url])
}

const installedPluginIds = computed(() => new Set(plugins.value.map((p) => p.manifest.id)))

const selectedHomeSet = computed<Set<string>>(() => {
  const sel = uiPreferences.value.home.selectedPlugins
  if (sel) return new Set(sel)
  return new Set(plugins.value.filter((p) => p.enabled).slice(0, 5).map((p) => p.manifest.id))
})

function resolveGroupName(entry: PluginListEntry): string {
  if (entry.repoId?.trim()) return entry.repoId.trim()
  // Backwards-compatible grouping for plugins installed before repoId persistence
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
  if (entry.manifest.id.startsWith('community.anthology')) {
    return 'Anthology'
  }
  if (entry.sourceType === 'dev_dir') {
    return 'Local Dev Plugins'
  }
  return 'Standalone Plugins'
}

interface PluginGroup {
  name: string
  items: PluginListEntry[]
  enabledCount: number
  homeCount: number
  iconUrl?: string
}

function resolveGroupIcon(items: PluginListEntry[]): string | undefined {
  const brandMatch = items.find((item) => {
    const icon = item.manifest.icon?.trim()
    return icon && (icon.includes('logo_1_transparent') || item.manifest.id === 'community.anthology.plus')
  })
  if (brandMatch?.manifest.icon) return brandMatch.manifest.icon.trim()

  const counts = new Map<string, number>()
  for (const item of items) {
    const icon = item.manifest.icon?.trim()
    if (!icon) continue
    counts.set(icon, (counts.get(icon) ?? 0) + 1)
  }
  let bestUrl: string | undefined
  let bestCount = 0
  for (const [url, count] of counts.entries()) {
    if (count > bestCount) {
      bestUrl = url
      bestCount = count
    }
  }
  return bestUrl
}

const groupedPlugins = computed<PluginGroup[]>(() => {
  const q = installedFilter.value.trim().toLowerCase()
  const buckets = new Map<string, PluginListEntry[]>()
  for (const entry of plugins.value) {
    const groupName = resolveGroupName(entry)
    if (
      q &&
      !entry.manifest.name.toLowerCase().includes(q) &&
      !entry.manifest.id.toLowerCase().includes(q) &&
      !groupName.toLowerCase().includes(q)
    ) {
      continue
    }
    const arr = buckets.get(groupName)
    if (arr) arr.push(entry)
    else buckets.set(groupName, [entry])
  }

  const out: PluginGroup[] = []
  for (const [name, items] of buckets.entries()) {
    out.push({
      name,
      items,
      enabledCount: items.filter((i) => i.enabled).length,
      homeCount: items.filter((i) => i.enabled && selectedHomeSet.value.has(i.manifest.id)).length,
      iconUrl: resolveGroupIcon(items),
    })
  }
  return out
})

const filteredRepoPlugins = computed(() => {
  if (resolvedPreview.value?.kind !== 'repository') return []
  const q = repoFilter.value.trim().toLowerCase()
  const list = resolvedPreview.value.repo.plugins
  if (!q) return list
  return list.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.id.toLowerCase().includes(q) ||
      (p.description ?? '').toLowerCase().includes(q),
  )
})

function isGroupExpanded(group: PluginGroup): boolean {
  if (installedFilter.value.trim().length > 0) return true
  if (Object.prototype.hasOwnProperty.call(expandedGroups.value, group.name)) {
    return expandedGroups.value[group.name]
  }
  // Start multi-plugin repositories collapsed under their single clean Plugin Repo header
  return group.items.length === 1
}

function toggleGroupExpanded(groupName: string, current: boolean): void {
  expandedGroups.value = { ...expandedGroups.value, [groupName]: !current }
}

async function saveHomeSelection(ids: string[]): Promise<void> {
  uiPreferences.value.home.selectedPlugins = Array.from(new Set(ids))
  invalidateHomeCache()
  await savePreferences(uiPreferences.value)
}

async function togglePluginHome(pluginId: string): Promise<void> {
  const next = new Set(selectedHomeSet.value)
  if (next.has(pluginId)) next.delete(pluginId)
  else next.add(pluginId)
  await saveHomeSelection(Array.from(next))
}

async function toggleGroupHome(group: PluginGroup, showAll: boolean): Promise<void> {
  const next = new Set(selectedHomeSet.value)
  for (const entry of group.items) {
    if (showAll && entry.enabled) next.add(entry.manifest.id)
    else next.delete(entry.manifest.id)
  }
  await saveHomeSelection(Array.from(next))
  pushToast('info', showAll ? `Pinned ${group.name} to Home` : `Cleared ${group.name} from Home`)
}

async function setGroupEnabled(group: PluginGroup, enabled: boolean): Promise<void> {
  for (const entry of group.items) {
    if (entry.enabled !== enabled) {
      await vesselApi.setPluginEnabled(entry.manifest.id, enabled)
    }
  }
  invalidateHomeCache()
  await refreshPlugins()
  pushToast('info', `${enabled ? 'Enabled' : 'Disabled'} all plugins in ${group.name}`)
}

async function removeGroup(group: PluginGroup): Promise<void> {
  if (confirmRemoveGroup.value !== group.name) {
    confirmRemoveGroup.value = group.name
    setTimeout(() => {
      if (confirmRemoveGroup.value === group.name) confirmRemoveGroup.value = ''
    }, 3000)
    return
  }
  confirmRemoveGroup.value = ''
  const nextHome = new Set(selectedHomeSet.value)
  for (const entry of group.items) {
    nextHome.delete(entry.manifest.id)
    await vesselApi.uninstallPlugin(entry.manifest.id)
  }
  if (uiPreferences.value.home.selectedPlugins) {
    await saveHomeSelection(Array.from(nextHome))
  } else {
    invalidateHomeCache()
  }
  await refreshPlugins()
  pushToast('info', `Removed '${group.name}'`)
}

async function refreshPlugins(): Promise<void> {
  if (!isTauriRuntime()) {
    loadingPlugins.value = false
    return
  }
  loadingPlugins.value = true
  try {
    plugins.value = await vesselApi.listPlugins()
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    loadingPlugins.value = false
  }
}

async function inspectSource(): Promise<void> {
  const raw = sourceInput.value.trim()
  if (!raw) return
  resolving.value = true
  repoFilter.value = ''
  try {
    resolvedPreview.value = await vesselApi.resolveSource(raw)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    resolving.value = false
  }
}

async function installSourceTarget(target: string, keepPreview = false, repoName?: string): Promise<void> {
  installingTarget.value = target
  try {
    const res = await vesselApi.installSource(target, repoName)
    if (!keepPreview) {
      resolvedPreview.value = null
      sourceInput.value = ''
    }
    const nextHome = new Set(selectedHomeSet.value)
    nextHome.add(res.pluginId)
    await saveHomeSelection(Array.from(nextHome))
    await refreshPlugins()
    pushToast('info', `Installed ${res.pluginId} (${res.badge})`)
  } catch (err) {
    pushToast('error', String(err))
  } finally {
    installingTarget.value = ''
  }
}

async function installAllRepoPlugins(): Promise<void> {
  if (resolvedPreview.value?.kind !== 'repository' || batchInstalling.value) return
  const repoName = resolvedPreview.value.repo.name
  const targets = filteredRepoPlugins.value.filter(
    (rp) => (rp.downloadUrl || rp.sourcePath) && !(rp.description ?? '').startsWith('[Disabled in repo]'),
  )
  if (!targets.length) return
  batchInstalling.value = true
  let okCount = 0
  let failCount = 0
  const starterHome: string[] = []

  for (let i = 0; i < targets.length; i++) {
    const rp = targets[i]
    const target = (rp.downloadUrl ?? rp.sourcePath)!
    installingTarget.value = target
    batchProgress.value = `${i + 1}/${targets.length}`
    try {
      const res = await vesselApi.installSource(target, repoName)
      okCount += 1
      if (starterHome.length < 5) {
        starterHome.push(res.pluginId)
      }
    } catch {
      failCount += 1
    }
  }
  installingTarget.value = ''
  batchProgress.value = ''
  batchInstalling.value = false

  const existingSel = uiPreferences.value.home.selectedPlugins
  if (!existingSel || existingSel.length === 0) {
    await saveHomeSelection(starterHome)
  } else {
    invalidateHomeCache()
  }
  await refreshPlugins()
  pushToast(
    failCount > 0 ? 'warn' : 'info',
    `Installed ${okCount} provider(s) under ${repoName}${failCount > 0 ? ` (${failCount} failed/blocked)` : ''}.`,
  )
}

async function toggleEnabled(entry: PluginListEntry): Promise<void> {
  try {
    await vesselApi.setPluginEnabled(entry.manifest.id, !entry.enabled)
    invalidateHomeCache()
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
    const nextHome = new Set(selectedHomeSet.value)
    nextHome.delete(pluginId)
    if (uiPreferences.value.home.selectedPlugins) {
      await saveHomeSelection(Array.from(nextHome))
    } else {
      invalidateHomeCache()
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
      <div>
        <h1 class="v-page-title">{{ t('extensions.title') }}</h1>
        <p class="v-muted header-sub">
          Installed repositories and plugins. Click a plugin bundle to expand or manage individual providers.
        </p>
      </div>
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
        placeholder="gh:falsisdev/anthology, stremio://..., https://.../manifest.json, or local folder path"
      />
      <button type="submit" class="v-btn v-btn-primary" :disabled="resolving">
        <Loader2 v-if="resolving" :size="15" class="spin" aria-hidden="true" />
        <span>{{ resolving ? t('common.loading') : t('extensions.resolve') }}</span>
      </button>
    </form>

    <div v-if="resolving" class="v-card preview-loading-card">
      <Loader2 :size="18" class="spin accent-icon" aria-hidden="true" />
      <span>{{ t('common.loading') }}</span>
    </div>

    <div v-else-if="resolvedPreview" class="v-card preview-card">
      <div class="preview-head">
        <div class="preview-title-wrap">
          <img
            v-if="
              resolvedPreview.kind === 'repository' && canShowImage(resolvedPreview.repo.iconUrl)
            "
            :src="resolvedPreview.repo.iconUrl"
            alt=""
            class="preview-repo-logo"
            @error="markImageFailed(resolvedPreview.repo.iconUrl)"
          />
          <img
            v-else-if="
              resolvedPreview.kind === 'package' && canShowImage(resolvedPreview.manifest.icon)
            "
            :src="resolvedPreview.manifest.icon"
            alt=""
            class="preview-repo-logo"
            @error="markImageFailed(resolvedPreview.manifest.icon)"
          />
          <strong>
            {{
              resolvedPreview.kind === 'repository'
                ? `${resolvedPreview.repo.name} (${resolvedPreview.repo.plugins.length})`
                : resolvedPreview.manifest.name
            }}
          </strong>
        </div>
        <div class="preview-head-actions">
          <span
            class="v-badge"
            :class="resolvedPreview.badge === 'official' ? 'v-badge-success' : 'v-badge-accent'"
          >
            <ShieldCheck :size="12" />
            {{ resolvedPreview.badge === 'official' ? t('extensions.official') : t('extensions.community') }}
          </span>
          <button
            type="button"
            class="v-btn v-btn-icon"
            aria-label="Close preview"
            @click="resolvedPreview = null"
          >
            <X :size="14" />
          </button>
        </div>
      </div>

      <template v-if="resolvedPreview.kind === 'package'">
        <p class="v-muted preview-meta">
          v{{ resolvedPreview.manifest.version }} · SHA-256:
          <code>{{ resolvedPreview.sha256.slice(0, 12) }}...</code>
        </p>
        <button
          class="v-btn v-btn-primary"
          :disabled="installingTarget !== ''"
          @click="installSourceTarget(sourceInput.trim(), false)"
        >
          <Download :size="15" /> {{ t('extensions.install_now') }}
        </button>
      </template>

      <template v-else>
        <div class="repo-toolbar">
          <input
            v-model="repoFilter"
            type="search"
            class="v-input repo-filter-input"
            :placeholder="t('extensions.filter_repo')"
          />
          <button
            type="button"
            class="v-btn v-btn-primary"
            :disabled="batchInstalling || !filteredRepoPlugins.length"
            @click="installAllRepoPlugins"
          >
            <Download :size="14" />
            {{ batchInstalling ? `${t('common.loading')} (${batchProgress})` : t('extensions.install_all') }}
          </button>
        </div>

        <div class="repo-list">
          <div
            v-for="rp in filteredRepoPlugins"
            :key="rp.id"
            class="repo-plugin-row"
          >
            <div class="repo-plugin-info">
              <div class="repo-plugin-title">
                <strong>{{ rp.name }}</strong>
                <span class="v-muted">v{{ rp.version }}</span>
                <span v-if="installedPluginIds.has(rp.id)" class="v-badge v-badge-success">
                  <Check :size="11" /> {{ t('extensions.installed_badge') }}
                </span>
              </div>
              <p v-if="rp.description" class="v-muted repo-plugin-desc">{{ rp.description }}</p>
            </div>
            <button
              v-if="rp.downloadUrl || rp.sourcePath"
              class="v-btn v-btn-primary v-btn-sm"
              :disabled="installingTarget === (rp.downloadUrl ?? rp.sourcePath) || batchInstalling"
              @click="
                installSourceTarget(
                  (rp.downloadUrl ?? rp.sourcePath)!,
                  true,
                  resolvedPreview.kind === 'repository' ? resolvedPreview.repo.name : undefined,
                )
              "
            >
              {{
                installingTarget === (rp.downloadUrl ?? rp.sourcePath)
                  ? t('common.loading')
                  : installedPluginIds.has(rp.id)
                    ? t('common.reinstall')
                    : t('common.install')
              }}
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

    <div v-if="loadingPlugins && !plugins.length" class="v-card preview-loading-card">
      <Loader2 :size="18" class="spin accent-icon" />
      <span>{{ t('common.loading') }}</span>
    </div>

    <div v-else-if="!plugins.length" class="v-card v-muted">{{ t('common.empty') }}</div>

    <template v-else>
      <div class="installed-toolbar">
        <input
          v-model="installedFilter"
          type="search"
          class="v-input"
          :placeholder="t('extensions.filter_installed')"
        />
        <span class="v-badge v-badge-accent">
          <Home :size="12" /> {{ selectedHomeSet.size }} on Home
        </span>
      </div>

      <div class="group-stack">
        <div v-for="group in groupedPlugins" :key="group.name" class="v-card repo-group-card">
          <div
            class="repo-group-head"
            role="button"
            tabindex="0"
            @click="toggleGroupExpanded(group.name, isGroupExpanded(group))"
            @keydown.enter="toggleGroupExpanded(group.name, isGroupExpanded(group))"
          >
            <div class="repo-group-title">
              <ChevronDown v-if="isGroupExpanded(group)" :size="18" class="group-chevron" />
              <ChevronRight v-else :size="18" class="group-chevron" />
              <span class="group-icon-box" :class="{ 'has-logo': canShowImage(group.iconUrl) }">
                <img
                  v-if="canShowImage(group.iconUrl)"
                  :src="group.iconUrl"
                  alt=""
                  loading="lazy"
                  decoding="async"
                  class="group-logo-img"
                  @error="markImageFailed(group.iconUrl)"
                />
                <Package v-else :size="17" class="group-icon" />
              </span>
              <div class="group-title-text">
                <strong>{{ group.name }}</strong>
                <span class="v-muted group-meta">
                  {{ group.enabledCount }} / {{ group.items.length }} active · {{ group.homeCount }} on Home
                </span>
              </div>
            </div>

            <div class="repo-group-actions" @click.stop>
              <button
                type="button"
                class="v-btn v-btn-sm"
                @click="toggleGroupHome(group, group.homeCount === 0)"
              >
                <Home :size="13" />
                {{ group.homeCount === 0 ? t('extensions.pin_home') : t('extensions.clear_home') }}
              </button>
              <button
                type="button"
                class="v-btn v-btn-sm"
                @click="setGroupEnabled(group, group.enabledCount < group.items.length)"
              >
                {{ group.enabledCount < group.items.length ? t('extensions.enable_all') : t('extensions.disable_all') }}
              </button>
              <button
                type="button"
                class="v-btn v-btn-sm v-btn-danger"
                @click="removeGroup(group)"
              >
                <Trash2 :size="13" />
                {{ confirmRemoveGroup === group.name ? t('common.confirm') : t('common.remove') }}
              </button>
            </div>
          </div>

          <div v-if="isGroupExpanded(group)" class="plugin-sublist">
            <div v-for="entry in group.items" :key="entry.manifest.id" class="plugin-subrow">
              <div class="plugin-info">
                <div class="plugin-head">
                  <img
                    v-if="canShowImage(entry.manifest.icon)"
                    :src="entry.manifest.icon"
                    alt=""
                    loading="lazy"
                    decoding="async"
                    class="plugin-mini-logo"
                    @error="markImageFailed(entry.manifest.icon)"
                  />
                  <strong>{{ entry.manifest.name }}</strong>
                  <span class="sub-ver v-muted">v{{ entry.manifest.version }}</span>
                  <span
                    class="status-dot"
                    :class="{ active: entry.enabled }"
                    :title="entry.enabled ? t('common.enabled') : t('common.disabled')"
                  />
                  <span
                    v-if="entry.allowedDomains.length"
                    class="domain-chip v-muted"
                    :title="entry.allowedDomains.join(', ')"
                  >
                    <Globe :size="11" /> {{ entry.allowedDomains.length }} domain(s)
                  </span>
                </div>
                <p v-if="entry.manifest.description" class="v-muted plugin-desc">
                  {{ entry.manifest.description }}
                </p>
              </div>

              <div class="plugin-actions">
                <label class="v-switch" :title="`Show ${entry.manifest.name} on Home screen`">
                  <input
                    type="checkbox"
                    class="v-switch-input"
                    :checked="selectedHomeSet.has(entry.manifest.id)"
                    :disabled="!entry.enabled"
                    @change="togglePluginHome(entry.manifest.id)"
                  />
                  <span class="v-switch-track" />
                  <span class="home-toggle-label">Home</span>
                </label>

                <button
                  v-if="entry.sourceType === 'dev_dir'"
                  class="v-btn v-btn-sm"
                  @click="installSourceTarget(entry.localPath, false)"
                >
                  <RefreshCw :size="13" /> Reload
                </button>
                <button
                  v-if="entry.manifest.settingsSchema.length"
                  class="v-btn v-btn-sm"
                  @click="activeSettingsManifest = entry.manifest"
                >
                  <Settings2 :size="13" />
                </button>
                <button class="v-btn v-btn-sm" @click="toggleEnabled(entry)">
                  {{ entry.enabled ? t('common.disable') : t('common.enable') }}
                </button>
                <button
                  type="button"
                  class="v-btn v-btn-icon v-btn-danger"
                  :title="confirmUninstallId === entry.manifest.id ? 'Confirm uninstall' : t('common.uninstall')"
                  @click="removePlugin(entry.manifest.id)"
                >
                  <Trash2 :size="13" aria-hidden="true" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.extensions-view,
.group-stack {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.header-sub {
  margin: 0.25rem 0 0;
  font-size: 0.9rem;
}

.source-bar,
.installed-toolbar {
  display: flex;
  gap: 0.65rem;
  align-items: center;
  flex-wrap: wrap;
}

.source-bar .v-input,
.installed-toolbar .v-input {
  flex: 1;
  min-width: 240px;
}

.preview-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-color: var(--vessel-accent);
}

.preview-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
}

.preview-title-wrap {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.preview-repo-logo {
  width: 34px;
  height: 34px;
  border-radius: 9px;
  object-fit: contain;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  padding: 2px;
  flex-shrink: 0;
}

.preview-head-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.preview-meta {
  margin: 0;
}

.repo-toolbar {
  display: flex;
  gap: 0.65rem;
  align-items: center;
  width: 100%;
}

.repo-filter-input {
  flex: 1;
}

.repo-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  width: 100%;
  max-height: 360px;
  overflow-y: auto;
  padding-right: 0.25rem;
}

.repo-plugin-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  padding: 0.55rem 0.75rem;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.repo-plugin-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.repo-plugin-title {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.repo-plugin-desc {
  margin: 0;
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.repo-group-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.repo-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  cursor: pointer;
  user-select: none;
}

.repo-group-title {
  display: flex;
  align-items: center;
  gap: 0.7rem;
}

.group-chevron {
  color: var(--vessel-text-muted);
}

.group-icon-box {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(203, 166, 247, 0.14);
  border: 1px solid rgba(203, 166, 247, 0.28);
  color: var(--vessel-accent);
  overflow: hidden;
  flex-shrink: 0;
}

.group-icon-box.has-logo {
  background: var(--vessel-surface-input);
  border-color: var(--vessel-border-strong);
  padding: 3px;
}

.group-logo-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: 6px;
}

.plugin-mini-logo {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  object-fit: contain;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
  padding: 1px;
  flex-shrink: 0;
}

.group-title-text {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.group-meta {
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
}

.repo-group-actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
}

.plugin-sublist {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  margin-left: 1rem;
  padding-left: 1rem;
  border-left: 2px solid rgba(203, 166, 247, 0.22);
}

.plugin-subrow {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.9rem;
  flex-wrap: wrap;
  padding: 0.55rem 0.8rem;
  border-radius: 10px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.plugin-info {
  display: flex;
  flex-direction: column;
  gap: 0.18rem;
  min-width: 0;
  flex: 1;
}

.plugin-head {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.sub-ver {
  font-size: 0.78rem;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.2);
}

.status-dot.active {
  background: var(--vessel-success);
  box-shadow: 0 0 8px rgba(166, 227, 161, 0.55);
}

.domain-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.74rem;
}

.plugin-desc {
  margin: 0;
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.home-toggle-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: var(--vessel-text-muted);
}

.preview-loading-card {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  font-weight: 600;
}

.accent-icon {
  color: var(--vessel-accent);
}

.spin {
  animation: extSpin 0.85s linear infinite;
}

@keyframes extSpin {
  to {
    transform: rotate(360deg);
  }
}
</style>
