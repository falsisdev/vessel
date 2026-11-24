package dev.falsis.vessel.data

import android.content.Context
import android.os.Environment
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.DYNAMIC_MATERIAL_THEME_ID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import uniffi.vessel_ffi.EngineEventListener
import uniffi.vessel_ffi.VesselMobileCore
import java.io.File

class VesselRepository private constructor(private val appContext: Context) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }

    private var core: VesselMobileCore? = null

    private val _initError = MutableStateFlow<String?>(null)
    val initError: StateFlow<String?> = _initError.asStateFlow()

    private val _isReady = MutableStateFlow(false)
    val isReady: StateFlow<Boolean> = _isReady.asStateFlow()

    private val _engineInfo = MutableStateFlow<EngineInfo?>(null)
    val engineInfo: StateFlow<EngineInfo?> = _engineInfo.asStateFlow()

    private val dynamicMaterialTheme = ThemeSpec(
        id = DYNAMIC_MATERIAL_THEME_ID,
        name = "Material You (Auto)",
        author = "Android",
        isDark = true,
    )

    private val _themes = MutableStateFlow(listOf(dynamicMaterialTheme, ThemeSpec()))
    val themes: StateFlow<List<ThemeSpec>> = _themes.asStateFlow()

    private val _activeTheme = MutableStateFlow(ThemeSpec())
    val activeTheme: StateFlow<ThemeSpec> = _activeTheme.asStateFlow()

    private val _uiPreferences = MutableStateFlow(UiPreferences())
    val uiPreferences: StateFlow<UiPreferences> = _uiPreferences.asStateFlow()

    private val _locale = MutableStateFlow("en")
    val locale: StateFlow<String> = _locale.asStateFlow()

    private val _plugins = MutableStateFlow<List<PluginListEntry>>(emptyList())
    val plugins: StateFlow<List<PluginListEntry>> = _plugins.asStateFlow()

    private val _searchEvents = MutableSharedFlow<SearchResultEvent>(extraBufferCapacity = 64)
    val searchEvents: SharedFlow<SearchResultEvent> = _searchEvents.asSharedFlow()

    private val _downloadProgress = MutableStateFlow<Map<String, DownloadProgressSnapshot>>(emptyMap())
    val downloadProgress: StateFlow<Map<String, DownloadProgressSnapshot>> = _downloadProgress.asStateFlow()

    private val _domainPrompts = MutableStateFlow<List<DomainPermissionPrompt>>(emptyList())
    val domainPrompts: StateFlow<List<DomainPermissionPrompt>> = _domainPrompts.asStateFlow()

    private val _toasts = MutableSharedFlow<String>(extraBufferCapacity = 32)
    val toasts: SharedFlow<String> = _toasts.asSharedFlow()

    private val prefsStore by lazy {
        appContext.getSharedPreferences("vessel_mobile_prefs", Context.MODE_PRIVATE)
    }

    suspend fun initialize(): Unit = withContext(Dispatchers.IO) {
        if (core != null) return@withContext
        val savedLocale = prefsStore.getString("locale", "en") ?: "en"
        _locale.value = savedLocale
        VesselI18n.init(appContext, savedLocale)

        val dataDir = File(appContext.filesDir, "vessel-data").apply { mkdirs() }
        val publicRoot = resolvePublicRootDir(appContext).apply { mkdirs() }

        try {
            val listener = object : EngineEventListener {
                override fun onEventJson(eventJson: String) {
                    handleRawEvent(eventJson)
                }
            }
            val instance = VesselMobileCore.open(
                rootDir = publicRoot.absolutePath,
                dataDir = dataDir.absolutePath,
                listener = listener,
            )
            core = instance
            refreshCoreState()
            _initError.value = null
            _isReady.value = true
        } catch (err: UnsatisfiedLinkError) {
            _initError.value =
                "Native Rust core library (libvessel_ffi.so) not found for this ABI. Build crates/vessel-ffi with cargo-ndk or open with compiled jniLibs."
            _isReady.value = true
        } catch (err: Throwable) {
            _initError.value = err.message ?: err.toString()
            _isReady.value = true
        }
    }

    private fun resolvePublicRootDir(context: Context): File {
        val ext = context.getExternalFilesDir(null)
        if (ext != null) return File(ext, "Vessel")
        val dl = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)
        return File(dl, "Vessel")
    }

    private fun handleRawEvent(raw: String) {
        val obj: JsonObject = try {
            json.parseToJsonElement(raw).jsonObject
        } catch (_: Throwable) {
            return
        }
        when (obj["type"]?.jsonPrimitive?.content) {
            "search_partial_result" -> {
                runCatching { json.decodeFromJsonElement<SearchResultEvent>(obj) }
                    .onSuccess { _searchEvents.tryEmit(it.copy(done = false)) }
            }
            "search_completed" -> {
                val sid = obj["searchId"]?.jsonPrimitive?.content ?: return
                _searchEvents.tryEmit(SearchResultEvent(searchId = sid, done = true))
            }
            "download_progress" -> {
                runCatching { json.decodeFromJsonElement<DownloadProgressSnapshot>(obj) }
                    .onSuccess { snap ->
                        _downloadProgress.value = _downloadProgress.value + (snap.downloadId to snap)
                    }
            }
            "domain_permission_requested" -> {
                runCatching { json.decodeFromJsonElement<DomainPermissionPrompt>(obj) }
                    .onSuccess { prompt ->
                        val cur = _domainPrompts.value
                        if (cur.none { it.pluginId == prompt.pluginId && it.domain == prompt.domain }) {
                            _domainPrompts.value = cur + prompt
                        }
                    }
            }
        }
    }

    fun dismissDomainPrompt(prompt: DomainPermissionPrompt) {
        _domainPrompts.value = _domainPrompts.value.filterNot {
            it.pluginId == prompt.pluginId && it.domain == prompt.domain
        }
    }

    suspend fun respondDomainPrompt(prompt: DomainPermissionPrompt, allow: Boolean, permanent: Boolean) {
        dismissDomainPrompt(prompt)
        if (allow) {
            grantPluginDomain(prompt.pluginId, prompt.domain, permanent)
        }
    }

    fun notify(message: String) {
        _toasts.tryEmit(message)
    }

    fun notifyError(err: Throwable, fallback: String) {
        if (err is kotlinx.coroutines.CancellationException) return
        _toasts.tryEmit(err.message ?: fallback)
    }

    private fun requireCore(): VesselMobileCore =
        core ?: throw IllegalStateException(_initError.value ?: "Vessel native core unavailable")

    suspend fun refreshCoreState() = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext
        runCatching {
            val info = json.decodeFromString<EngineInfo>(c.getEngineInfoJson())
            _engineInfo.value = info
            VesselI18n.init(appContext, _locale.value, info.localesDir)
        }
        runCatching {
            val rawList = json.decodeFromString<List<ThemeSpec>>(c.listThemesJson())
            val list = buildList {
                add(dynamicMaterialTheme)
                rawList.filterNotTo(this) { it.id == DYNAMIC_MATERIAL_THEME_ID }
            }
            _themes.value = list
            val savedThemeId = prefsStore.getString("theme_id", "catppuccin-mocha")
            _activeTheme.value = list.firstOrNull { it.id == savedThemeId } ?: list.firstOrNull() ?: ThemeSpec()
        }
        runCatching {
            _uiPreferences.value = json.decodeFromString<UiPreferences>(c.getUiPreferencesJson())
        }
        runCatching {
            _plugins.value = json.decodeFromString<List<PluginListEntry>>(c.listPluginsJson())
        }
    }

    fun setLocale(code: String) {
        _locale.value = code
        prefsStore.edit().putString("locale", code).apply()
        VesselI18n.init(appContext, code, _engineInfo.value?.localesDir)
    }

    fun selectTheme(themeId: String) {
        prefsStore.edit().putString("theme_id", themeId).apply()
        _activeTheme.value = _themes.value.firstOrNull { it.id == themeId } ?: ThemeSpec()
    }

    suspend fun getHomeFeeds(pluginIds: List<String>? = null): List<HomeShelfResult> = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getHomeFeedsJson(pluginIds))
    }

    suspend fun searchStreams(query: String, pluginIds: List<String>? = null): String = withContext(Dispatchers.IO) {
        requireCore().searchStreams(query, null, 1u, pluginIds)
    }

    suspend fun cancelSearch() = withContext(Dispatchers.IO) {
        core?.cancelSearch()
    }

    suspend fun getDetails(pluginId: String, nativeId: String): MediaDetail = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getDetailsJson(pluginId, nativeId, _locale.value))
    }

    suspend fun getStreams(pluginId: String, episodeId: String): StreamResult = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getStreamsJson(pluginId, episodeId))
    }

    suspend fun setMetadataOverride(
        pluginId: String,
        nativeId: String,
        canonicalId: String?,
        isDisabled: Boolean,
    ) = withContext(Dispatchers.IO) {
        requireCore().setMetadataOverride(pluginId, nativeId, canonicalId, isDisabled)
    }

    suspend fun getSkipSegments(payload: GetSkipSegmentsPayload): List<SkipSegment> = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getSkipSegmentsJson(json.encodeToString(payload)))
    }

    suspend fun preparePlaybackUrl(candidate: StreamCandidate): String = withContext(Dispatchers.IO) {
        requireCore().preparePlaybackUrl(json.encodeToString(candidate))
    }

    suspend fun recordProgress(progress: PlaybackProgress) = withContext(Dispatchers.IO) {
        requireCore().recordProgressJson(json.encodeToString(progress))
    }

    suspend fun listContinueWatching(limit: Int = 20): List<ContinueWatchingEntry> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.listContinueWatchingJson(limit.toUInt()))
    }

    suspend fun listMediaWatchProgress(pluginId: String, nativeId: String): List<PlaybackProgress> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.listMediaWatchProgressJson(pluginId, nativeId))
    }

    suspend fun deleteWatchProgress(pluginId: String, nativeId: String) = withContext(Dispatchers.IO) {
        requireCore().deleteWatchProgress(pluginId, nativeId)
    }

    suspend fun listLibrary(status: String? = null): List<LibraryItem> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.listLibraryJson(status))
    }

    suspend fun upsertLibrary(item: LibraryItem) = withContext(Dispatchers.IO) {
        requireCore().upsertLibraryJson(json.encodeToString(item))
    }

    suspend fun deleteLibraryItem(pluginId: String, nativeId: String) = withContext(Dispatchers.IO) {
        requireCore().deleteLibraryItem(pluginId, nativeId)
    }

    suspend fun listPlugins(): List<PluginListEntry> = withContext(Dispatchers.IO) {
        val list = json.decodeFromString<List<PluginListEntry>>(requireCore().listPluginsJson())
        _plugins.value = list
        list
    }

    suspend fun resolveSource(rawInput: String): ResolvedSource = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().resolveSourceJson(rawInput))
    }

    suspend fun installSource(rawInput: String, repoId: String? = null): InstallResult = withContext(Dispatchers.IO) {
        val res = json.decodeFromString<InstallResult>(requireCore().installSourceJson(rawInput, repoId))
        refreshCoreState()
        val curSel = _uiPreferences.value.home.selectedPlugins
        if (curSel != null) {
            val nextSel = if (curSel.isEmpty()) null else (curSel + res.pluginId).distinct()
            runCatching {
                saveUiPreferences(_uiPreferences.value.copy(home = _uiPreferences.value.home.copy(selectedPlugins = nextSel)))
            }
        }
        res
    }

    suspend fun setPluginEnabled(pluginId: String, enabled: Boolean) = withContext(Dispatchers.IO) {
        requireCore().setPluginEnabled(pluginId, enabled)
        refreshCoreState()
    }

    suspend fun uninstallPlugin(pluginId: String) = withContext(Dispatchers.IO) {
        requireCore().uninstallPlugin(pluginId)
        refreshCoreState()
    }

    suspend fun grantPluginDomain(pluginId: String, domain: String, permanent: Boolean) = withContext(Dispatchers.IO) {
        requireCore().grantPluginDomain(pluginId, domain, permanent)
        refreshCoreState()
    }

    suspend fun listDownloads(): List<StoredDownload> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.listDownloadsJson())
    }

    suspend fun enqueueDownload(payload: EnqueueDownloadPayload): StoredDownload = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().enqueueDownloadJson(json.encodeToString(payload)))
    }

    suspend fun pauseDownload(downloadId: String) = withContext(Dispatchers.IO) {
        requireCore().pauseDownload(downloadId)
    }

    suspend fun resumeDownload(downloadId: String): StoredDownload = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().resumeDownloadJson(downloadId))
    }

    suspend fun deleteDownload(downloadId: String) = withContext(Dispatchers.IO) {
        requireCore().deleteDownload(downloadId)
        _downloadProgress.value = _downloadProgress.value - downloadId
    }

    suspend fun reorderDownloads(orderedIds: List<String>) = withContext(Dispatchers.IO) {
        requireCore().reorderDownloads(orderedIds)
    }

    suspend fun prepareLocalDownloadPlayback(downloadId: String): LocalPlaybackBundle = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().prepareLocalDownloadPlaybackJson(downloadId))
    }

    suspend fun listIptvProfiles(): List<StoredIptvProfile> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.listIptvProfilesJson())
    }

    suspend fun upsertIptvProfile(payload: UpsertIptvProfilePayload): StoredIptvProfile = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().upsertIptvProfileJson(json.encodeToString(payload)))
    }

    suspend fun syncIptvProfile(profileId: String): Int = withContext(Dispatchers.IO) {
        requireCore().syncIptvProfile(profileId).toInt()
    }

    suspend fun syncIptvEpg(profileId: String): Int = withContext(Dispatchers.IO) {
        requireCore().syncIptvEpg(profileId).toInt()
    }

    suspend fun searchIptvChannels(
        profileId: String? = null,
        query: String? = null,
        groupTitle: String? = null,
        favoritesOnly: Boolean = false,
    ): List<StoredIptvChannel> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        json.decodeFromString(c.searchIptvChannelsJson(profileId, query, groupTitle, favoritesOnly, 250u))
    }

    suspend fun setIptvFavorite(channelId: Long, isFavorite: Boolean) = withContext(Dispatchers.IO) {
        requireCore().setIptvFavorite(channelId, isFavorite)
    }

    suspend fun playIptvChannel(channel: StoredIptvChannel, userAgent: String?): String = withContext(Dispatchers.IO) {
        requireCore().playIptvChannel(json.encodeToString(channel), userAgent)
    }

    suspend fun deleteIptvProfile(profileId: String) = withContext(Dispatchers.IO) {
        requireCore().deleteIptvProfile(profileId)
    }

    suspend fun listIptvGroups(profileId: String): List<String> = withContext(Dispatchers.IO) {
        val c = core ?: return@withContext emptyList()
        c.listIptvGroups(profileId)
    }

    suspend fun getEpgNowNext(profileId: String, tvgId: String): EpgNowNext = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getEpgNowNextJson(profileId, tvgId, null))
    }

    suspend fun clearRuntimeCaches() = withContext(Dispatchers.IO) {
        core?.clearRuntimeCaches()
    }

    suspend fun getPluginSettingsMap(pluginId: String): Map<String, JsonElement> = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().getPluginSettingsJson(pluginId))
    }

    suspend fun setPluginSettingsMap(pluginId: String, values: Map<String, JsonElement>) = withContext(Dispatchers.IO) {
        requireCore().setPluginSettingsJson(pluginId, json.encodeToString(values))
    }

    suspend fun importThemeJson(themeJson: String): ThemeSpec = withContext(Dispatchers.IO) {
        val spec = json.decodeFromString<ThemeSpec>(requireCore().importThemeJson(themeJson))
        refreshCoreState()
        selectTheme(spec.id)
        spec
    }

    suspend fun setAlwaysProxy(enabled: Boolean) = withContext(Dispatchers.IO) {
        requireCore().setAlwaysProxy(enabled)
        refreshCoreState()
    }

    suspend fun setMetadataEnabled(enabled: Boolean) = withContext(Dispatchers.IO) {
        requireCore().setMetadataEnabled(enabled)
        refreshCoreState()
    }

    suspend fun setTmdbApiKey(apiKey: String?) = withContext(Dispatchers.IO) {
        requireCore().setTmdbApiKey(apiKey)
    }

    suspend fun saveUiPreferences(prefs: UiPreferences) = withContext(Dispatchers.IO) {
        requireCore().setUiPreferencesJson(json.encodeToString(prefs))
        _uiPreferences.value = prefs
    }

    suspend fun resetUiPreferences() = saveUiPreferences(UiPreferences())

    suspend fun exportBackupJson(includeSecrets: Boolean): String = withContext(Dispatchers.IO) {
        requireCore().exportBackupJson(includeSecrets)
    }

    suspend fun importBackupJson(rawJson: String, overwrite: Boolean) = withContext(Dispatchers.IO) {
        requireCore().importBackupJson(rawJson, if (overwrite) "reset_and_import" else "smart_merge")
        refreshCoreState()
    }

    companion object {
        @Volatile
        private var instance: VesselRepository? = null

        fun get(context: Context): VesselRepository {
            return instance ?: synchronized(this) {
                instance ?: VesselRepository(context.applicationContext).also { instance = it }
            }
        }
    }
}
