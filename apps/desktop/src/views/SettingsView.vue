<script setup lang="ts">
import { computed, onMounted, shallowRef } from 'vue'
import { Download, ExternalLink, Palette, RefreshCw, RotateCcw, Shield, Sliders, Sparkles, Upload } from 'lucide-vue-next'
import { isTauriRuntime, vesselApi } from '@/api/vessel'
import { formatBytes, useAppUpdater } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'
import { useTheme } from '@/composables/useTheme'
import { useUiPreferences } from '@/composables/useUiPreferences'
import { pushToast } from '@/composables/useVesselEvents'
import type { EngineInfo } from '@/types/vessel'

const { t, currentLang, availableLocales, getLocaleLabel, setLocale } = useI18n()
const { themes, activeThemeId, selectTheme, importTheme } = useTheme()
const { uiPreferences, savePreferences, resetAllPreferences } = useUiPreferences()
const {
  updateInfo,
  parsedNotes,
  checkingUpdate,
  downloadingUpdate,
  downloadProgress,
  autoCheckEnabled,
  loadUpdatePreferences,
  checkForUpdates,
  toggleAutoCheck,
  downloadAndInstallUpdate,
} = useAppUpdater()

const engineInfo = shallowRef<EngineInfo | null>(null)
const alwaysProxy = shallowRef(false)
const metadataEnabled = shallowRef(true)
const tmdbApiKey = shallowRef('')
const themeJsonInput = shallowRef('')
const backupJsonInput = shallowRef('')
const includeSecretsInBackup = shallowRef(false)
const confirmResetImport = shallowRef(false)

const subtitlePreviewLineStyle = computed(() => {
  const subs = uiPreferences.value.subtitles
  const alpha = Math.max(0, Math.min(100, subs.backgroundOpacity ?? 72)) / 100
  const scaledSize = Math.round(Math.max(12, Math.min(34, (subs.fontSizePx ?? 28) * 0.68)))
  const fontFamilyMap: Record<string, string> = {
    'system-ui': 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    serif: 'Georgia, "Times New Roman", serif',
    monospace: '"JetBrains Mono", "SFMono-Regular", Menlo, monospace',
    rounded: '"Nunito", "SF Pro Rounded", system-ui, sans-serif',
  }
  const family = fontFamilyMap[subs.fontFamily] ?? (subs.fontFamily || fontFamilyMap['system-ui'])
  const outlineStyle = subs.outlineStyle ?? 'outline'
  const textShadow =
    outlineStyle === 'none'
      ? 'none'
      : outlineStyle === 'shadow'
        ? '0 2px 10px rgba(0, 0, 0, 0.95), 0 1px 3px rgba(0, 0, 0, 0.9)'
        : '0 2px 6px rgba(0,0,0,0.95), -1.5px -1.5px 0 rgba(0,0,0,0.88), 1.5px -1.5px 0 rgba(0,0,0,0.88), -1.5px 1.5px 0 rgba(0,0,0,0.88), 1.5px 1.5px 0 rgba(0,0,0,0.88)'
  return {
    fontSize: `${scaledSize}px`,
    fontFamily: family,
    color: subs.textColor || '#ffffff',
    fontWeight: subs.bold ? '700' : '500',
    backgroundColor: outlineStyle === 'none' ? 'transparent' : `rgba(8, 10, 16, ${alpha})`,
    textShadow,
  }
})

const subtitlePreviewStageOffset = computed(() => {
  const raw = uiPreferences.value.subtitles.bottomOffsetPx ?? 56
  return `${Math.max(8, Math.min(54, Math.round(raw * 0.24)))}px`
})

onMounted(async () => {
  if (!isTauriRuntime()) return
  try {
    const [info] = await Promise.all([vesselApi.getEngineInfo(), loadUpdatePreferences()])
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
          <strong>{{ t('settings.theme_label') }}</strong>
          <span class="v-muted">{{ t('settings.theme_desc') }}</span>
        </div>
        <select
          :aria-label="t('settings.theme_label')"
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
          <strong>{{ t('settings.locale_label') }}</strong>
          <span class="v-muted">{{ t('settings.locale_desc') }}</span>
        </div>
        <select
          :aria-label="t('settings.locale_label')"
          :value="currentLang"
          class="v-select v-setting-control"
          @change="setLocale(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="code in availableLocales" :key="code" :value="code">
            {{ getLocaleLabel(code) }}
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
        <Upload :size="14" aria-hidden="true" /> {{ t('settings.import_theme') }}
      </button>
    </div>

    <div class="v-card section-card">
      <div class="section-title-row">
        <Sliders :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.ui_customization') }}</h3>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.nav_layout') }}</strong>
          <span class="v-muted">{{ t('settings.nav_layout_desc') }}</span>
        </div>
        <select
          v-model="uiPreferences.shell.layout"
          :aria-label="t('settings.nav_layout')"
          class="v-select v-setting-control"
        >
          <option value="expanded_sidebar">{{ t('settings.layout_floating') }}</option>
          <option value="pinned_sidebar">{{ t('settings.layout_pinned') }}</option>
          <option value="floating_bottom">{{ t('settings.layout_floating_bottom') }}</option>
          <option value="collapsed_rail">{{ t('settings.layout_compact') }}</option>
          <option value="top_navbar">{{ t('settings.layout_top') }}</option>
          <option value="hidden">{{ t('settings.layout_hidden') }}</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.density') }}</strong>
          <span class="v-muted">{{ t('settings.density_desc') }}</span>
        </div>
        <select
          v-model="uiPreferences.shell.density"
          :aria-label="t('settings.density')"
          class="v-select v-setting-control"
        >
          <option value="compact">{{ t('settings.density_compact') }}</option>
          <option value="comfortable">{{ t('settings.density_comfortable') }}</option>
          <option value="spacious">{{ t('settings.density_spacious') }}</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.corner_style') }}</strong>
          <span class="v-muted">{{ t('settings.corner_desc') }}</span>
        </div>
        <select
          v-model="uiPreferences.shell.cornerStyle"
          :aria-label="t('settings.corner_style')"
          class="v-select v-setting-control"
        >
          <option value="sharp">{{ t('settings.corner_sharp') }}</option>
          <option value="rounded">{{ t('settings.corner_rounded') }}</option>
          <option value="pill">{{ t('settings.corner_pill') }}</option>
        </select>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.play_click') }}</strong>
          <span class="v-muted">{{ t('settings.play_click_desc') }}</span>
        </div>
        <select
          v-model="uiPreferences.playback.clickBehavior"
          :aria-label="t('settings.play_click')"
          class="v-select v-setting-control"
        >
          <option value="quick_start_best">{{ t('settings.play_quick') }}</option>
          <option value="always_ask_source">{{ t('settings.play_ask') }}</option>
        </select>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>{{ t('settings.show_plugin_badge') }}</strong></span>
        <label class="v-switch">
          <input v-model="uiPreferences.cards.showPluginBadge" type="checkbox" class="v-switch-input" />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>{{ t('settings.show_rating_chip') }}</strong></span>
        <label class="v-switch">
          <input v-model="uiPreferences.cards.showRating" type="checkbox" class="v-switch-input" />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="v-setting-row">
        <span class="setting-copy"><strong>{{ t('settings.spoiler_free') }}</strong></span>
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
        <span class="setting-copy"><strong>{{ t('settings.custom_css') }}</strong></span>
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

      <div class="sub-prefs-box">
        <h4 class="sub-prefs-title">{{ t('settings.subtitles_title') }}</h4>

        <div class="sub-preview-stage" :style="{ paddingBottom: subtitlePreviewStageOffset }">
          <span class="sub-preview-badge">{{ t('subtitles.preview_title') }}</span>
          <span class="sub-preview-line" :style="subtitlePreviewLineStyle">
            {{ t('subtitles.preview_sample') }}
          </span>
        </div>

        <div class="v-setting-row">
          <div class="setting-copy">
            <strong>{{ t('subtitles.font_size') }}</strong>
            <span class="v-muted">{{ uiPreferences.subtitles.fontSizePx }}px</span>
          </div>
          <input
            v-model.number="uiPreferences.subtitles.fontSizePx"
            type="range"
            min="16"
            max="52"
            step="2"
            class="v-setting-control sub-range"
          />
        </div>

        <div class="v-setting-row">
          <div class="setting-copy">
            <strong>{{ t('subtitles.bottom_offset') }}</strong>
            <span class="v-muted">{{ uiPreferences.subtitles.bottomOffsetPx }}px</span>
          </div>
          <input
            v-model.number="uiPreferences.subtitles.bottomOffsetPx"
            type="range"
            min="16"
            max="220"
            step="4"
            class="v-setting-control sub-range"
          />
        </div>

        <div class="v-setting-row">
          <div class="setting-copy">
            <strong>{{ t('subtitles.bg_opacity') }}</strong>
            <span class="v-muted">{{ uiPreferences.subtitles.backgroundOpacity }}%</span>
          </div>
          <input
            v-model.number="uiPreferences.subtitles.backgroundOpacity"
            type="range"
            min="0"
            max="100"
            step="5"
            class="v-setting-control sub-range"
          />
        </div>

        <div class="v-setting-row">
          <span class="setting-copy"><strong>{{ t('subtitles.font_family') }}</strong></span>
          <select
            v-model="uiPreferences.subtitles.fontFamily"
            :aria-label="t('subtitles.font_family')"
            class="v-select v-setting-control"
          >
            <option value="system-ui">System Sans</option>
            <option value="rounded">Rounded</option>
            <option value="serif">Serif</option>
            <option value="monospace">Monospace</option>
          </select>
        </div>

        <div class="v-setting-row">
          <span class="setting-copy"><strong>{{ t('subtitles.outline_style') }}</strong></span>
          <select
            v-model="uiPreferences.subtitles.outlineStyle"
            :aria-label="t('subtitles.outline_style')"
            class="v-select v-setting-control"
          >
            <option value="outline">{{ t('subtitles.style_outline') }}</option>
            <option value="shadow">{{ t('subtitles.style_shadow') }}</option>
            <option value="box">{{ t('subtitles.style_box') }}</option>
            <option value="none">{{ t('subtitles.style_none') }}</option>
          </select>
        </div>

        <div class="v-setting-row">
          <span class="setting-copy"><strong>{{ t('subtitles.text_color') }}</strong></span>
          <div class="sub-color-row v-setting-control">
            <button
              v-for="clr in ['#ffffff', '#fde047', '#67e8f9', '#f9a8d4', '#a7f3d0']"
              :key="clr"
              type="button"
              class="sub-color-swatch"
              :style="{ backgroundColor: clr }"
              :class="{ active: uiPreferences.subtitles.textColor.toLowerCase() === clr }"
              :aria-label="clr"
              @click="uiPreferences.subtitles.textColor = clr"
            />
          </div>
        </div>

        <div class="v-setting-row">
          <span class="setting-copy"><strong>{{ t('subtitles.bold') }}</strong></span>
          <label class="v-switch">
            <input v-model="uiPreferences.subtitles.bold" type="checkbox" class="v-switch-input" />
            <span class="v-switch-track" aria-hidden="true" />
          </label>
        </div>
      </div>

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
          <span class="v-muted">{{ t('settings.metadata_desc') }}</span>
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
        <button type="button" class="v-btn" @click="saveTmdbKey">{{ t('settings.save_api_key') }}</button>
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
          <strong>{{ t('settings.backup_secrets') }}</strong>
          <span class="v-muted">{{ t('settings.backup_secrets_desc') }}</span>
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
          {{ confirmResetImport ? t('common.confirm') : t('settings.import_reset') }}
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

    <div class="v-card section-card">
      <div class="section-title-row">
        <Sparkles :size="18" class="section-icon" aria-hidden="true" />
        <h3>{{ t('settings.updates_title') }}</h3>
      </div>

      <div class="v-setting-row">
        <div class="setting-copy">
          <strong>{{ t('settings.updates_auto') }}</strong>
          <span class="v-muted">{{ t('settings.updates_auto_desc') }}</span>
        </div>
        <label class="v-switch">
          <input
            :checked="autoCheckEnabled"
            type="checkbox"
            class="v-switch-input"
            @change="toggleAutoCheck(!autoCheckEnabled)"
          />
          <span class="v-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div class="inline-row">
        <button
          type="button"
          class="v-btn v-btn-primary"
          :disabled="checkingUpdate"
          @click="checkForUpdates(false)"
        >
          <RefreshCw :size="14" :class="{ spin: checkingUpdate }" aria-hidden="true" />
          {{ checkingUpdate ? t('settings.checking_updates') : t('settings.check_updates') }}
        </button>
        <span class="v-muted update-status-line">
          <template v-if="updateInfo">
            Current: <code>v{{ updateInfo.currentVersion }}</code> · Latest:
            <code>v{{ updateInfo.latestVersion }}</code>
          </template>
          <template v-else>Repository: <code>falsisdev/vessel</code></template>
        </span>
      </div>

      <div v-if="updateInfo" class="release-notes-box">
        <div class="release-notes-head">
          <strong>
            {{ updateInfo.releaseName || `Vessel ${updateInfo.tagName}` }}
          </strong>
          <span
            class="v-badge"
            :class="updateInfo.updateAvailable ? 'v-badge-accent' : 'v-badge-success'"
          >
            {{ updateInfo.updateAvailable ? t('settings.update_available') : t('settings.up_to_date') }}
          </span>
        </div>

        <div class="release-notes-list">
          <template v-for="(line, idx) in parsedNotes" :key="idx">
            <h4 v-if="line.kind === 'heading'" class="release-note-h">{{ line.text }}</h4>
            <div v-else-if="line.kind === 'bullet'" class="release-note-li">
              • {{ line.text }}
            </div>
            <p v-else class="release-note-p">{{ line.text }}</p>
          </template>
        </div>

        <div v-if="downloadingUpdate && downloadProgress" class="update-progress-wrap">
          <div class="inline-row">
            <span class="v-muted">
              Downloading {{ downloadProgress.fileName }} ({{ downloadProgress.percent ?? 0 }}%)
            </span>
          </div>
          <div class="update-progress-track">
            <div
              class="update-progress-fill"
              :style="{ width: `${downloadProgress.percent ?? 8}%` }"
            />
          </div>
        </div>

        <div class="inline-row">
          <button
            v-if="updateInfo.updateAvailable && updateInfo.downloadUrl"
            type="button"
            class="v-btn v-btn-primary v-btn-sm"
            :disabled="downloadingUpdate"
            @click="downloadAndInstallUpdate"
          >
            <Download :size="14" aria-hidden="true" />
            Download &amp; Install {{ updateInfo.assetName }}
            <template v-if="updateInfo.assetSizeBytes">
              ({{ formatBytes(updateInfo.assetSizeBytes) }})
            </template>
          </button>
          <button
            type="button"
            class="v-btn v-btn-sm"
            @click="vesselApi.openExternalUrl(updateInfo.htmlUrl)"
          >
            <ExternalLink :size="14" aria-hidden="true" />
            {{ t('settings.open_releases') }}
          </button>
        </div>
      </div>
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

.update-status-line {
  font-size: 0.84rem;
}

.release-notes-box {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.9rem 1rem;
  border-radius: 12px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.sub-prefs-box {
  width: 100%;
  margin-top: 0.5rem;
  padding-top: 0.85rem;
  border-top: 1px solid var(--vessel-border);
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.sub-prefs-title {
  margin: 0 0 0.5rem;
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--vessel-accent);
}

.sub-preview-stage {
  position: relative;
  min-height: 112px;
  margin-bottom: 0.65rem;
  padding: 2.2rem 1rem 0.85rem;
  border-radius: 12px;
  border: 1px solid var(--vessel-border);
  background:
    radial-gradient(circle at 25% 25%, rgba(203, 166, 247, 0.2), transparent 55%),
    radial-gradient(circle at 80% 35%, rgba(137, 180, 250, 0.16), transparent 50%),
    linear-gradient(180deg, #161829 0%, #090a12 100%);
  display: flex;
  align-items: flex-end;
  justify-content: center;
  text-align: center;
  overflow: hidden;
  transition: padding-bottom 0.15s ease;
}

.sub-preview-badge {
  position: absolute;
  top: 0.5rem;
  left: 0.65rem;
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: rgba(244, 246, 252, 0.68);
}

.sub-preview-line {
  display: inline-block;
  max-width: 92%;
  padding: 0.28em 0.68em;
  border-radius: 8px;
  line-height: 1.35;
}

.sub-range {
  accent-color: var(--vessel-accent);
  cursor: pointer;
}

.sub-color-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.5rem;
}

.sub-color-swatch {
  width: 24px;
  height: 24px;
  border-radius: 999px;
  border: 2px solid var(--vessel-border-strong);
  cursor: pointer;
  transition: transform 0.16s ease, border-color 0.16s ease;
}

.sub-color-swatch.active {
  transform: scale(1.18);
  border-color: var(--vessel-accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--vessel-accent) 35%, transparent);
}

.release-notes-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.release-notes-list {
  display: flex;
  flex-direction: column;
  gap: 0.32rem;
  max-height: 200px;
  overflow-y: auto;
}

.release-note-h {
  margin: 0.25rem 0 0;
  font-size: 0.88rem;
  color: var(--vessel-accent);
}

.release-note-li,
.release-note-p {
  margin: 0;
  font-size: 0.84rem;
  color: var(--vessel-text-muted);
}

.update-progress-wrap {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.update-progress-track {
  width: 100%;
  height: 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.14);
  overflow: hidden;
}

.update-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
  transition: width 0.18s ease;
}

.spin {
  animation: v-spin 0.9s linear infinite;
}

@keyframes v-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
