import { computed, ref, shallowRef } from 'vue'
import { isTauriRuntime, listenUpdateProgress, vesselApi } from '@/api/vessel'
import { pushToast } from '@/composables/useVesselEvents'
import type { AppUpdateInfo, UpdateDownloadProgress } from '@/types/vessel'

const updateInfo = ref<AppUpdateInfo | null>(null)
const checkingUpdate = shallowRef(false)
const downloadingUpdate = shallowRef(false)
const downloadProgress = ref<UpdateDownloadProgress | null>(null)
const installedPath = shallowRef<string>('')
const showUpdateModal = shallowRef(false)
const autoCheckEnabled = shallowRef(true)
const lastCheckError = shallowRef('')

let progressUnlisten: (() => void) | null = null

export interface ParsedNoteLine {
  kind: 'heading' | 'bullet' | 'text'
  text: string
}

export function parseReleaseNotes(markdown: string): ParsedNoteLine[] {
  const lines = markdown
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
  if (!lines.length) {
    return [{ kind: 'text', text: 'Bug fixes and stability improvements.' }]
  }
  return lines.map((line) => {
    if (line.startsWith('#')) {
      return { kind: 'heading', text: line.replace(/^#+\s*/, '') }
    }
    if (line.startsWith('- ') || line.startsWith('* ')) {
      return { kind: 'bullet', text: line.slice(2).trim() }
    }
    return { kind: 'text', text: line }
  })
}

export function formatBytes(bytes?: number): string {
  if (!bytes || bytes <= 0) return ''
  const mb = bytes / (1024 * 1024)
  if (mb >= 1) return `${mb.toFixed(1)} MB`
  return `${Math.round(bytes / 1024)} KB`
}

export function useAppUpdater() {
  const parsedNotes = computed(() => parseReleaseNotes(updateInfo.value?.releaseNotes ?? ''))

  async function loadUpdatePreferences(): Promise<void> {
    if (!isTauriRuntime()) return
    try {
      const prefs = await vesselApi.getUpdatePreferences()
      autoCheckEnabled.value = prefs.autoCheckEnabled
    } catch {
      // Non-fatal
    }
  }

  async function checkForUpdates(silent = false): Promise<AppUpdateInfo | null> {
    if (!isTauriRuntime() || checkingUpdate.value) return null
    checkingUpdate.value = true
    lastCheckError.value = ''
    try {
      const info = await vesselApi.checkAppUpdate()
      updateInfo.value = info
      autoCheckEnabled.value = info.autoCheckEnabled

      if (info.updateAvailable) {
        const skipped = info.skippedVersion?.replace(/^v/i, '')
        const latest = info.latestVersion.replace(/^v/i, '')
        if (!silent || skipped !== latest) {
          showUpdateModal.value = true
        }
      } else if (!silent) {
        pushToast('info', `Vessel v${info.currentVersion} is up to date.`)
      }
      return info
    } catch (err) {
      lastCheckError.value = String(err)
      if (!silent) {
        pushToast('warn', String(err))
      }
      return null
    } finally {
      checkingUpdate.value = false
    }
  }

  async function toggleAutoCheck(enabled: boolean): Promise<void> {
    autoCheckEnabled.value = enabled
    if (updateInfo.value) {
      updateInfo.value = { ...updateInfo.value, autoCheckEnabled: enabled }
    }
    if (!isTauriRuntime()) return
    try {
      await vesselApi.setAutoCheckUpdates(enabled)
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  async function skipUpdateVersion(): Promise<void> {
    const version = updateInfo.value?.latestVersion
    showUpdateModal.value = false
    if (!version || !isTauriRuntime()) return
    try {
      await vesselApi.skipAppUpdateVersion(version)
      if (updateInfo.value) {
        updateInfo.value = { ...updateInfo.value, skippedVersion: version }
      }
      pushToast('info', `Skipped notifications for v${version}`)
    } catch (err) {
      pushToast('error', String(err))
    }
  }

  async function downloadAndInstallUpdate(): Promise<void> {
    const info = updateInfo.value
    if (!info || !info.downloadUrl || !info.assetName || downloadingUpdate.value) return
    downloadingUpdate.value = true
    installedPath.value = ''
    downloadProgress.value = {
      fileName: info.assetName,
      downloadedBytes: 0,
      totalBytes: info.assetSizeBytes,
      percent: 0,
    }

    if (!progressUnlisten) {
      progressUnlisten = await listenUpdateProgress((p) => {
        downloadProgress.value = p
      })
    }

    try {
      const savedPath = await vesselApi.downloadAndOpenUpdate(info.downloadUrl, info.assetName)
      installedPath.value = savedPath
      pushToast('info', `Downloaded & opened ${info.assetName}`)
    } catch (err) {
      pushToast('error', String(err))
    } finally {
      downloadingUpdate.value = false
      if (progressUnlisten) {
        progressUnlisten()
        progressUnlisten = null
      }
    }
  }

  function dismissModal(): void {
    showUpdateModal.value = false
  }

  function openModal(): void {
    showUpdateModal.value = true
  }

  return {
    updateInfo,
    parsedNotes,
    checkingUpdate,
    downloadingUpdate,
    downloadProgress,
    installedPath,
    showUpdateModal,
    autoCheckEnabled,
    lastCheckError,
    loadUpdatePreferences,
    checkForUpdates,
    toggleAutoCheck,
    skipUpdateVersion,
    downloadAndInstallUpdate,
    dismissModal,
    openModal,
  }
}
