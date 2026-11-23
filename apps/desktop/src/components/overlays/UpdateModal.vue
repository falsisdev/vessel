<script setup lang="ts">
import { Download, ExternalLink, Sparkles, X } from 'lucide-vue-next'
import { formatBytes, useAppUpdater } from '@/composables/useAppUpdater'
import { useI18n } from '@/composables/useI18n'

const { t } = useI18n()

const {
  updateInfo,
  parsedNotes,
  downloadingUpdate,
  downloadProgress,
  installedPath,
  showUpdateModal,
  downloadAndInstallUpdate,
  skipUpdateVersion,
  dismissModal,
} = useAppUpdater()
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-pop">
      <div
        v-if="showUpdateModal && updateInfo?.updateAvailable"
        class="update-backdrop"
        role="dialog"
        aria-modal="true"
        aria-label="Vessel Update Available"
        @click.self="dismissModal"
      >
        <div class="v-card update-modal">
          <div class="update-head">
            <div class="update-title-group">
              <span class="update-badge">
                <Sparkles :size="14" aria-hidden="true" />
                {{ t('settings.update_available') }}
              </span>
              <h2 class="update-heading">
                {{ updateInfo.releaseName || `Vessel ${updateInfo.tagName}` }}
              </h2>
              <p class="v-muted update-ver-line">
                v{{ updateInfo.currentVersion }} → <strong>v{{ updateInfo.latestVersion }}</strong>
                <template v-if="updateInfo.assetSizeBytes">
                  · {{ formatBytes(updateInfo.assetSizeBytes) }}
                </template>
              </p>
            </div>
            <button
              type="button"
              class="v-btn v-btn-icon"
              aria-label="Close"
              @click="dismissModal"
            >
              <X :size="16" />
            </button>
          </div>

          <div class="notes-panel">
            <div class="notes-label">{{ t('overlay.patch_notes') }}</div>
            <div class="notes-scroll">
              <template v-for="(line, idx) in parsedNotes" :key="idx">
                <h4 v-if="line.kind === 'heading'" class="note-heading">{{ line.text }}</h4>
                <div v-else-if="line.kind === 'bullet'" class="note-bullet">
                  <span class="note-dot" aria-hidden="true" />
                  <span>{{ line.text }}</span>
                </div>
                <p v-else class="note-text">{{ line.text }}</p>
              </template>
            </div>
          </div>

          <div v-if="downloadingUpdate && downloadProgress" class="progress-box">
            <div class="progress-labels">
              <span>Downloading {{ downloadProgress.fileName }}…</span>
              <span>{{ downloadProgress.percent ?? 0 }}%</span>
            </div>
            <div class="progress-track">
              <div
                class="progress-fill"
                :style="{ width: `${downloadProgress.percent ?? 8}%` }"
              />
            </div>
          </div>

          <p v-else-if="installedPath" class="v-muted saved-note">
            Installer opened from <code>{{ installedPath }}</code>
          </p>

          <div class="update-actions">
            <button type="button" class="v-btn v-btn-sm" @click="skipUpdateVersion">
              Skip v{{ updateInfo.latestVersion }}
            </button>
            <a
              :href="updateInfo.htmlUrl"
              target="_blank"
              rel="noopener noreferrer"
              class="v-btn v-btn-sm"
            >
              <ExternalLink :size="14" aria-hidden="true" />
              GitHub Release
            </a>
            <button
              v-if="updateInfo.downloadUrl"
              type="button"
              class="v-btn v-btn-primary"
              :disabled="downloadingUpdate"
              @click="downloadAndInstallUpdate"
            >
              <Download :size="15" aria-hidden="true" />
              {{ downloadingUpdate ? 'Downloading…' : `Install (${updateInfo.assetName})` }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.update-backdrop {
  position: fixed;
  inset: 0;
  z-index: 120;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: rgba(6, 7, 12, 0.78);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.update-modal {
  width: min(560px, 100%);
  display: flex;
  flex-direction: column;
  gap: 1rem;
  border-color: rgba(203, 166, 247, 0.38);
}

.update-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.update-title-group {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.update-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  width: fit-content;
  padding: 0.2rem 0.65rem;
  border-radius: 999px;
  font-size: 0.74rem;
  font-weight: 700;
  background: rgba(203, 166, 247, 0.18);
  color: var(--vessel-accent);
  border: 1px solid rgba(203, 166, 247, 0.38);
}

.update-heading {
  margin: 0;
  font-size: 1.32rem;
  font-weight: 800;
  letter-spacing: -0.02em;
}

.update-ver-line {
  margin: 0;
  font-size: 0.86rem;
  font-variant-numeric: tabular-nums;
}

.notes-panel {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.9rem 1rem;
  border-radius: 12px;
  background: var(--vessel-surface-input);
  border: 1px solid var(--vessel-border);
}

.notes-label {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--vessel-text-muted);
}

.notes-scroll {
  max-height: 220px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding-right: 0.25rem;
}

.note-heading {
  margin: 0.3rem 0 0.1rem;
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--vessel-accent);
}

.note-bullet {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  font-size: 0.86rem;
  line-height: 1.45;
}

.note-dot {
  width: 6px;
  height: 6px;
  margin-top: 0.45rem;
  border-radius: 999px;
  background: var(--vessel-accent-secondary);
  flex-shrink: 0;
}

.note-text {
  margin: 0;
  font-size: 0.86rem;
  line-height: 1.45;
  color: var(--vessel-text-muted);
}

.progress-box {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.progress-labels {
  display: flex;
  justify-content: space-between;
  font-size: 0.8rem;
  font-variant-numeric: tabular-nums;
}

.progress-track {
  height: 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.14);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--vessel-accent-secondary), var(--vessel-accent));
  transition: width 0.18s ease;
}

.saved-note {
  margin: 0;
  font-size: 0.8rem;
}

.update-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.55rem;
}
</style>
