import type { MediaSummary, PluginListEntry } from '@/types/vessel'

/**
 * Classifies whether an installed plugin belongs exclusively (or functionally)
 * to the Live TV section rather than the VOD/Anime Home shelves.
 */
export function isLiveTvPlugin(p: PluginListEntry): boolean {
  const caps = p.manifest.capabilities ?? []
  if (caps.includes('live_tv')) return true
  const cts = (p.manifest.contentTypes ?? []).map((c) => c.toLowerCase())
  if (cts.includes('live')) return true
  const hay = `${p.manifest.id} ${p.manifest.name} ${p.manifest.description ?? ''}`.toLowerCase()
  return (
    hay.includes('iptv') ||
    hay.includes('canlı tv') ||
    hay.includes('canli tv') ||
    hay.includes('live tv') ||
    hay.includes('livetv') ||
    hay.includes('_spor')
  )
}

export function resolveLiveGroupName(entry: PluginListEntry): string {
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

export function isLiveMediaIdentifier(
  mediaType?: string | null,
  pluginId?: string | null,
  streamId?: string | null,
  nativeId?: string | null,
  plugin?: PluginListEntry | null,
): boolean {
  if (mediaType?.toLowerCase() === 'live') return true
  const pid = pluginId?.toLowerCase() ?? ''
  if (pid === 'iptv') return true
  if (plugin && isLiveTvPlugin(plugin)) return true
  const sid = streamId?.toLowerCase() ?? ''
  const nid = nativeId?.toLowerCase() ?? ''
  return (
    sid.startsWith('tv:') ||
    sid.startsWith('tv_') ||
    sid.startsWith('iptv_') ||
    sid.startsWith('vns::tv::') ||
    nid.startsWith('tv:') ||
    nid.startsWith('tv_') ||
    nid.startsWith('iptv_')
  )
}

export function isLiveMediaSummary(item: MediaSummary, plugin?: PluginListEntry | null): boolean {
  return isLiveMediaIdentifier(item.type, item.pluginId, item.nativeId, item.nativeId, plugin)
}
