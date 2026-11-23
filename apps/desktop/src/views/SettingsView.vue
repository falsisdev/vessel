<script setup lang="ts">
import { onMounted, shallowRef } from 'vue'
import { Download, Palette, RotateCcw, Shield, Sliders, Upload } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { useTheme } from '@/composables/useTheme'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { EngineInfo } from '@/types/vessel'

const { t, currentLang, availableLocales, setLocale } = useI18n()
const { themes, activeThemeId, selectTheme, importTheme } = useTheme()
const { uiPreferences, savePreferences, resetAllPreferences } = useUiPreferences()

const engineInfo = shallowRef<EngineInfo | null>(null)
const alwaysProxy = shallowRef(false)
const metadataEnabled = shallowRef(true)
const tmdbApiKey = shallowRef('')
const themeJsonInput = shallowRef('')
const backupJsonInput = shallowRef('')
const includeSecretsInBackup = shallowRef(false)
const confirmResetImport = shallowRef(false)

onMounted(async () => {
  if (!isTauriRuntime()) return
  try {
    const info = await vesselApi.getEngineInfo()
    engineInfo.value = info
    alwaysProxy.value = info.alwaysProxy
    metadataEnabled.value = info.metadataEnabled
  } catch (err) {
    pushToast('error', String(err))
  }
})

async function toggleAlwaysProxy(): Promise<void> {
  alwaysProxy.value = !alwaysProxy.value
  await vesselApi.setAlwaysProxy(alwaysProxy.value)
}

async function toggleMetadataEnabled(): Promise<void> {
  metadataEnabled.value = !metadataEnabled.value
  await vesselApi.setMetadataEnabled(metadataEnabled.value)
}

async function saveTmdbKey(): Promise<void> {
  try {
    await vesselApi.setTmdbApiKey(tmdbApiKey.value)
    tmdbApiKey.value = ''
    pushToast('info', 'Saved TMDB API key to SecretStore')
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleImportTheme(): Promise<void> {
  if (!themeJsonInput.value.trim()) return
  try {
    const spec = await importTheme(themeJsonInput.value)
    themeJsonInput.value = ''
    pushToast('info', `Imported theme ${spec.name}`)
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function persistUi(): Promise<void> {
  try {
    await savePreferences({ ...uiPreferences.value })
    pushToast('info', 'UI preferences saved')
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleExportBackup(): Promise<void> {
  try {
    backupJsonInput.value = await vesselApi.exportBackupJson(includeSecretsInBackup.value)
    pushToast('info', 'Backup JSON exported below')
  } catch (err) {
    pushToast('error', String(err))
  }
}

async function handleImportBackup(mode: 'smart_merge' | 'reset_and_import'): Promise<void> {
  if (!backupJsonInput.value.trim()) return
  if (mode === 'reset_and_import' && !confirmResetImport.value) {
    confirmResetImport.value = true
    setTimeout(() => {
      confirmResetImport.value = false
    }, 3500)
    return
  }
  confirmResetImport.value = false
  try {
    await vesselApi.importBackupJson(backupJsonInput.value, mode)
    pushToast('info', `Imported backup (${mode})`)
  } catch (err) {
    pushToast('error', String(err))
  }
}
</script>

<template>
  <section class="settings-view v-page-container">
    <header class="v-page-header">
      <h1 class="v-page-title">{{ t('settings.title') }}</h1>
      <button type="button" class="v-btn v-btn-sm" @click="resetAllPreferences">
        <RotateCcw :size="14" aria-hidden="true" /> {{ t('settings.reset_ui') }}
      </button>
    </header>

    <div class="v-card section-card">
      <div class="section-title-row">
        <Palette :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.appearance') }} &amp; {{ t('settings.language') }}</h3>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Theme</strong>
          <span class="v-muted">Choose built-in palette or import custom .theme.vessel JSON</span>
        </div>
        <select
          aria-label="Theme"
          :value="activeThemeId"
          class="v-select v-setting-control"
          @change="selectTheme(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="th in themes" :key="th.id" :value="th.id">
            {{ th.name }} ({{ th.author }})
          </option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Locale</strong>
          <span class="v-muted">Loaded from bundled English or &lt;VesselRoot&gt;/locales</span>
        </div>
        <select
          aria-label="Locale"
          :value="currentLang"
          class="v-select v-setting-control"
          @change="setLocale(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="code in availableLocales" :key="code" :value="code">
            {{ code }}
          </option>
        </select>
      </div>

      <textarea
        v-model="themeJsonInput"
        name="themeJson"
        aria-label="Theme JSON"
        rows="3"
        class="v-textarea"
        placeholder="Paste .theme.vessel JSON here…"
      />
      <button type="button" class="v-btn v-btn-sm" @click="handleImportTheme">
        <Upload :size="14" aria-hidden="true" /> Import .theme.vessel
      </button>
    </div>

    <div class="v-card section-card">
      <div class="section-title-row">
        <Sliders :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.ui_customization') }}</h3>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Navigation Layout</strong>
          <span class="v-muted">Choose rail, expanded sidebar, top bar, or minimal mode</span>
        </div>
        <select
          v-model="uiPreferences.shell.layout"
          aria-label="Navigation Layout"
          class="v-select v-setting-control"
        >
          <option value="expanded_sidebar">Floating Expand Rail</option>
          <option value="collapsed_rail">Compact Icon Rail</option>
          <option value="top_navbar">Top Navbar</option>
          <option value="hidden">Hidden</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Interface Density</strong>
          <span class="v-muted">Adjust spacing rhythm across grids and rails</span>
        </div>
        <select
          v-model="uiPreferences.shell.density"
          aria-label="Interface Density"
          class="v-select v-setting-control"
        >
          <option value="compact">Compact</option>
          <option value="comfortable">Comfortable</option>
          <option value="spacious">Spacious</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Corner Style</strong>
          <span class="v-muted">Poster and panel roundness</span>
        </div>
        <select
          v-model="uiPreferences.shell.cornerStyle"
          aria-label="Corner Style"
          class="v-select v-setting-control"
        >
          <option value="sharp">Sharp</option>
          <option value="rounded">Rounded</option>
          <option value="pill">Pill</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>Play Click Behavior</strong>
          <span class="v-muted">Auto-start best stream or ask source picker</span>
        </div>
        <select
          v-model="uiPreferences.playback.clickBehavior"
          aria-label="Play Click Behavior"
          class="v-select v-setting-control"
        >
          <option value="quick_start_best">Quick Start Best</option>
          <option value="always_ask_source">Always Ask Source</option>
        </select>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>Show Plugin Source on Cards</strong></span>
        <label class="v-switch">
          <input v-model="uiPreferences.cards.showPluginBadge" type="checkbox" class="v-switch-input" />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>Show Rating Chip</strong></span>
        <label class="v-switch">
          <input v-model="uiPreferences.cards.showRating" type="checkbox" class="v-switch-input" />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>Spoiler-Free Episode Overviews</strong></span>
        <label class="v-switch">
          <input
            v-model="uiPreferences.details.hideEpisodeOverviews"
            type="checkbox"
            class="v-switch-input"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>Enable Custom CSS Injection</strong></span>
        <label class="v-switch">
          <input
            v-model="uiPreferences.advanced.customCssEnabled"
            type="checkbox"
            class="v-switch-input"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <textarea
        v-if="uiPreferences.advanced.customCssEnabled"
        v-model="uiPreferences.advanced.customCss"
        aria-label="Custom CSS"
        rows="4"
        class="v-textarea"
        placeholder="/* Custom CSS injected safely on desktop */"
      />

      <button type="button" class="v-btn v-btn-primary" @click="persistUi">
        {{ t('common.save') }}
      </button>
    </div>

    <div class="v-card section-card">
      <div class="section-title-row">
        <Shield :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.playback_privacy') }}</h3>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.always_proxy') }}</strong>
          <span class="v-muted">Route direct MP4/MKV streams through local 127.0.0.1 proxy</span>
        </div>
        <label class="v-switch">
          <input
            :checked="alwaysProxy"
            type="checkbox"
            class="v-switch-input"
            @change="toggleAlwaysProxy"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.metadata_enabled') }}</strong>
          <span class="v-muted">Turn off for strict plugin-only zero external queries</span>
        </div>
        <label class="v-switch">
          <input
            :checked="metadataEnabled"
            type="checkbox"
            class="v-switch-input"
            @change="toggleMetadataEnabled"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="inline-row">
        <input
          v-model="tmdbApiKey"
          type="password"
          name="tmdbApiKey"
          autocomplete="off"
          aria-label="TMDB API Key"
          class="v-input"
          :placeholder="t('settings.tmdb_key')"
        />
        <button type="button" class="v-btn" @click="saveTmdbKey">Save API Key</button>
      </div>

      <div v-if="engineInfo" class="v-muted paths-info">
        <span>Root: <code>{{ engineInfo.rootDir }}</code></span> ·
        <span>Proxy: <code>{{ engineInfo.proxyBaseUrl }}</code></span>
      </div>
    </div>

    <div class="v-card section-card">
      <div class="section-title-row">
        <Download :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.backup') }}</h3>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy">
          <strong>Include Stored Secrets in Export</strong>
          <span class="v-muted">Omitted by default for safe sharing</span>
        </span>
        <label class="v-switch">
          <input v-model="includeSecretsInBackup" type="checkbox" class="v-switch-input" />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="inline-row">
        <button type="button" class="v-btn" @click="handleExportBackup">
          {{ t('settings.export_backup') }}
        </button>
        <button type="button" class="v-btn v-btn-primary" @click="handleImportBackup('smart_merge')">
          {{ t('settings.import_smart_merge') }}
        </button>
        <button type="button" class="v-btn v-btn-danger" @click="handleImportBackup('reset_and_import')">
          {{ confirmResetImport ? 'Confirm Full Replace' : t('settings.import_reset') }}
        </button>
      </div>

      <textarea
        v-model="backupJsonInput"
        name="backupJson"
        aria-label="Backup JSON"
        rows="5"
        class="v-textarea"
        placeholder="Exported or pasted .backup.vessel JSON…"
      />
    </div>
  </section>
</template>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  gap: var(--vessel-gap);
}

.section-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
}

.section-title-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.25rem;
}

.section-title-row h3 {
  margin: 0;
  font-size: 1.08rem;
}

.section-icon {
  color: var(--vessel-accent);
}

.setting-copy {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  min-width: 0;
}

.setting-copy span {
  font-size: 0.82rem;
}

.inline-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  width: 100%;
  margin-top: 0.35rem;
}

.inline-row .v-input {
  flex: 1;
  min-width: 220px;
}

.paths-info {
  font-size: 0.82rem;
}
</style>
