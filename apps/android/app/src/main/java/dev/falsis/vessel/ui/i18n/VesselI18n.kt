package dev.falsis.vessel.ui.i18n

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import java.io.File

object VesselI18n {
    val bundledCodes = listOf("en", "tr", "es", "de", "fr", "it", "ru", "ja", "pt", "zh", "ar", "fa")

    private val json = Json { ignoreUnknownKeys = true }
    private var englishMap: Map<String, String> = emptyMap()
    private var activeMap: Map<String, String> = emptyMap()

    /** Snapshot-observed revision counter so all composables recompose immediately on locale switch. */
    var revision by mutableIntStateOf(0)
        private set

    fun init(context: Context, locale: String, customLocalesDir: String? = null) {
        if (englishMap.isEmpty()) {
            englishMap = loadAssetLocale(context, "en")
        }
        activeMap = if (locale == "en") {
            englishMap
        } else {
            val fromDisk = customLocalesDir
                ?.let { File(it, "$locale.json") }
                ?.takeIf { it.isFile }
                ?.let { flattenJson(it.readText()) }
            fromDisk ?: loadAssetLocale(context, locale).ifEmpty { englishMap }
        }
        revision++
    }

    fun tr(key: String, fallback: String? = null): String {
        @Suppress("UNUSED_VARIABLE")
        val rev = revision
        return activeMap[key] ?: englishMap[key] ?: fallback ?: key
    }

    fun trf(key: String, vararg args: Pair<String, Any?>, fallback: String? = null): String {
        var text = tr(key, fallback)
        for ((name, value) in args) {
            text = text.replace("{$name}", value?.toString().orEmpty())
        }
        return text
    }

    fun isRtl(locale: String): Boolean = locale == "ar" || locale == "fa"

    private fun loadAssetLocale(context: Context, code: String): Map<String, String> {
        return try {
            context.assets.open("locales/$code.json").bufferedReader().use { reader ->
                flattenJson(reader.readText())
            }
        } catch (_: Throwable) {
            emptyMap()
        }
    }

    private fun flattenJson(raw: String): Map<String, String> {
        val out = mutableMapOf<String, String>()
        val root = try {
            json.parseToJsonElement(raw) as? JsonObject ?: return out
        } catch (_: Throwable) {
            return out
        }
        fun walk(prefix: String, obj: JsonObject) {
            for ((k, v) in obj) {
                val path = if (prefix.isEmpty()) k else "$prefix.$k"
                when (v) {
                    is JsonObject -> walk(path, v)
                    is JsonPrimitive -> out[path] = v.content
                    else -> {}
                }
            }
        }
        walk("", root)
        return out
    }
}
