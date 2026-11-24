package dev.falsis.vessel.ui

import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.PluginListEntry

fun isLiveTvPlugin(p: PluginListEntry): Boolean {
    val caps = p.manifest.capabilities
    if ("live_tv" in caps) return true
    val cts = p.manifest.contentTypes.map { it.lowercase() }
    if ("live" in cts) return true
    val hay = "${p.manifest.id} ${p.manifest.name} ${p.manifest.description ?: ""}".lowercase()
    return "iptv" in hay ||
        "canlı tv" in hay ||
        "canli tv" in hay ||
        "live tv" in hay ||
        "livetv" in hay ||
        "_spor" in hay
}

fun resolveLiveGroupName(entry: PluginListEntry): String {
    val rid = entry.repoId?.trim()
    if (!rid.isNullOrEmpty()) return rid
    if (
        "raw.githubusercontent.com" in entry.allowedDomains &&
        (entry.manifest.author == "falsisdev" ||
            entry.manifest.author == "metemel" ||
            entry.manifest.id.startsWith("anthology") ||
            entry.manifest.id == "m3u.anthology.addon")
    ) {
        return "Anthology"
    }
    if (entry.manifest.id.startsWith("community.anthology")) return "Anthology"
    if (entry.sourceType == "dev_dir") return "Local Dev Plugins"
    return "Standalone Plugins"
}

fun isLiveMediaIdentifier(
    mediaType: String?,
    pluginId: String?,
    streamId: String?,
    nativeId: String?,
    plugin: PluginListEntry? = null,
): Boolean {
    if (mediaType.equals("live", ignoreCase = true)) return true
    if (pluginId.equals("iptv", ignoreCase = true)) return true
    if (plugin != null && isLiveTvPlugin(plugin)) return true
    val sid = streamId?.lowercase().orEmpty()
    val nid = nativeId?.lowercase().orEmpty()
    return sid.startsWith("tv:") ||
        sid.startsWith("tv_") ||
        sid.startsWith("iptv_") ||
        sid.startsWith("vns::tv::") ||
        nid.startsWith("tv:") ||
        nid.startsWith("tv_") ||
        nid.startsWith("iptv_")
}

fun isLiveMediaSummary(item: MediaSummary, plugin: PluginListEntry? = null): Boolean =
    isLiveMediaIdentifier(item.type, item.pluginId, item.nativeId, item.nativeId, plugin)
