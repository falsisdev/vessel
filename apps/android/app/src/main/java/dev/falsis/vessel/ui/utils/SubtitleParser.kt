package dev.falsis.vessel.ui.utils

data class SubtitleCue(
    val startMs: Long,
    val endMs: Long,
    val lines: List<String>,
)

private val ASS_TAG_REGEX = Regex("\\{\\\\[^}]*\\}")
private val HTML_TAG_REGEX = Regex("<[^>]+>")
private val BLANK_LINES_REGEX = Regex("\\n{2,}")
private val WHITESPACE_REGEX = Regex("\\s+")

private fun parseTimestampToMs(raw: String): Long {
    val cleaned = raw.trim().replace(',', '.')
    val parts = cleaned.split(':')
    if (parts.size !in 2..3) return 0L

    var hours = 0L
    val mins: Long
    val secsRaw: String
    if (parts.size == 3) {
        hours = parts[0].toLongOrNull() ?: 0L
        mins = parts[1].toLongOrNull() ?: 0L
        secsRaw = parts[2]
    } else {
        mins = parts[0].toLongOrNull() ?: 0L
        secsRaw = parts[1]
    }
    val secs = secsRaw.toDoubleOrNull() ?: 0.0
    return kotlin.math.max(0L, Math.round((hours * 3600.0 + mins * 60.0 + secs) * 1000.0))
}

private fun stripMarkupTags(text: String): String {
    return text
        .replace(ASS_TAG_REGEX, "")
        .replace(HTML_TAG_REGEX, "")
        .replace("\\N", "\n")
        .replace("\\n", "\n")
        .replace("\\h", " ")
        .trim()
}

private fun parseSrtOrVtt(rawText: String): List<SubtitleCue> {
    val normalized = rawText.removePrefix("\uFEFF").replace("\r\n", "\n").replace("\r", "\n")
    val blocks = normalized.split(BLANK_LINES_REGEX)
    val cues = mutableListOf<SubtitleCue>()

    for (block in blocks) {
        val rawLines = block.split('\n').map { it.trimEnd() }
        val timingIdx = rawLines.indexOfFirst { it.contains("-->") }
        if (timingIdx == -1) continue

        val arrowParts = rawLines[timingIdx].split("-->")
        if (arrowParts.size < 2) continue

        val startMs = parseTimestampToMs(arrowParts[0])
        val endToken = arrowParts[1].trim().split(WHITESPACE_REGEX).firstOrNull().orEmpty()
        val endMs = parseTimestampToMs(endToken)
        if (endMs <= startMs) continue

        val bodyLines = rawLines
            .drop(timingIdx + 1)
            .map(::stripMarkupTags)
            .flatMap { it.split('\n') }
            .map { it.trim() }
            .filter { it.isNotEmpty() }

        if (bodyLines.isNotEmpty()) {
            cues += SubtitleCue(startMs = startMs, endMs = endMs, lines = bodyLines)
        }
    }

    return cues.sortedBy { it.startMs }
}

private fun parseAssOrSsa(rawText: String): List<SubtitleCue> {
    val normalized = rawText.removePrefix("\uFEFF").replace("\r\n", "\n").replace("\r", "\n")
    val cues = mutableListOf<SubtitleCue>()
    var formatFields = listOf("layer", "start", "end", "style", "name", "marginl", "marginr", "marginv", "effect", "text")
    var inEvents = false

    for (rawLine in normalized.split('\n')) {
        val line = rawLine.trim()
        if (line.isEmpty()) continue
        if (line.startsWith('[')) {
            inEvents = line.equals("[events]", ignoreCase = true)
            continue
        }
        if (!inEvents && !line.startsWith("Dialogue:", ignoreCase = true)) continue

        if (line.startsWith("Format:", ignoreCase = true)) {
            formatFields = line.substring(7).split(',').map { it.trim().lowercase() }
            continue
        }

        if (line.startsWith("Dialogue:", ignoreCase = true)) {
            val payload = line.substring(9).trim()
            val parts = payload.split(',')
            val textIdx = maxOf(formatFields.indexOf("text"), 9)
            if (parts.size <= textIdx) continue

            val startIdx = maxOf(formatFields.indexOf("start"), 1)
            val endIdx = maxOf(formatFields.indexOf("end"), 2)
            val startMs = parseTimestampToMs(parts.getOrNull(startIdx).orEmpty())
            val endMs = parseTimestampToMs(parts.getOrNull(endIdx).orEmpty())
            if (endMs <= startMs) continue

            val rawTextPart = parts.drop(textIdx).joinToString(",")
            val cleanLines = stripMarkupTags(rawTextPart)
                .split('\n')
                .map { it.trim() }
                .filter { it.isNotEmpty() }
            if (cleanLines.isNotEmpty()) {
                cues += SubtitleCue(startMs = startMs, endMs = endMs, lines = cleanLines)
            }
        }
    }

    return cues.sortedBy { it.startMs }
}

fun parseSubtitles(rawText: String, format: String? = null): List<SubtitleCue> {
    val trimmed = rawText.trimStart()
    return if (
        format.equals("ass", ignoreCase = true) ||
        format.equals("ssa", ignoreCase = true) ||
        trimmed.contains("[Script Info]") ||
        trimmed.contains("Dialogue:")
    ) {
        parseAssOrSsa(rawText)
    } else {
        parseSrtOrVtt(rawText)
    }
}

fun findActiveSubtitleLines(cues: List<SubtitleCue>, targetMs: Long): List<String> {
    if (cues.isEmpty()) return emptyList()
    var lo = 0
    var hi = cues.lastIndex
    var pos = cues.size
    while (lo <= hi) {
        val mid = (lo + hi) ushr 1
        if (cues[mid].endMs >= targetMs) {
            pos = mid
            hi = mid - 1
        } else {
            lo = mid + 1
        }
    }
    val out = mutableListOf<String>()
    for (i in pos until cues.size) {
        val cue = cues[i]
        if (cue.startMs > targetMs) break
        if (targetMs in cue.startMs..cue.endMs) {
            out += cue.lines
        }
    }
    return out
}
