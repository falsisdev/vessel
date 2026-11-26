package dev.falsis.vessel.data

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import androidx.core.content.FileProvider
import dev.falsis.vessel.BuildConfig
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
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import uniffi.vessel_ffi.EngineEventListener
import uniffi.vessel_ffi.VesselMobileCore
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.time.Instant

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

    private val _metadataEnriched = MutableSharedFlow<MetadataEnrichedEvent>(replay = 8, extraBufferCapacity = 32)
    val metadataEnriched: SharedFlow<MetadataEnrichedEvent> = _metadataEnriched.asSharedFlow()

    private val _downloadProgress = MutableStateFlow<Map<String, DownloadProgressSnapshot>>(emptyMap())
    val downloadProgress: StateFlow<Map<String, DownloadProgressSnapshot>> = _downloadProgress.asStateFlow()

    private val _domainPrompts = MutableStateFlow<List<DomainPermissionPrompt>>(emptyList())
    val domainPrompts: StateFlow<List<DomainPermissionPrompt>> = _domainPrompts.asStateFlow()

    private val _toasts = MutableSharedFlow<String>(extraBufferCapacity = 32)
    val toasts: SharedFlow<String> = _toasts.asSharedFlow()

    private val _autoCheckUpdates = MutableStateFlow(true)
    val autoCheckUpdates: StateFlow<Boolean> = _autoCheckUpdates.asStateFlow()

    private val _skippedUpdateVersion = MutableStateFlow<String?>(null)
    val skippedUpdateVersion: StateFlow<String?> = _skippedUpdateVersion.asStateFlow()

    private val _updateInfo = MutableStateFlow<AppUpdateInfo?>(null)
    val updateInfo: StateFlow<AppUpdateInfo?> = _updateInfo.asStateFlow()

    private val _checkingUpdate = MutableStateFlow(false)
    val checkingUpdate: StateFlow<Boolean> = _checkingUpdate.asStateFlow()

    private val _downloadingUpdate = MutableStateFlow(false)
    val downloadingUpdate: StateFlow<Boolean> = _downloadingUpdate.asStateFlow()

    private val _updateDownloadPercent = MutableStateFlow<Int?>(null)
    val updateDownloadPercent: StateFlow<Int?> = _updateDownloadPercent.asStateFlow()

    private val _updateError = MutableStateFlow<String?>(null)
    val updateError: StateFlow<String?> = _updateError.asStateFlow()

    private val _showUpdateDialog = MutableStateFlow(false)
    val showUpdateDialog: StateFlow<Boolean> = _showUpdateDialog.asStateFlow()

    private val prefsStore by lazy {
        appContext.getSharedPreferences("vessel_mobile_prefs", Context.MODE_PRIVATE)
    }

    suspend fun initialize(): Unit = withContext(Dispatchers.IO) {
        if (core != null) return@withContext
        val savedLocale = prefsStore.getString("locale", "en") ?: "en"
        _autoCheckUpdates.value = prefsStore.getBoolean("auto_check_updates", true)
        _skippedUpdateVersion.value = prefsStore.getString("skipped_update_version", null)
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
            "metadata_enriched" -> {
                runCatching { json.decodeFromJsonElement<MetadataEnrichedEvent>(obj) }
                    .onSuccess { _metadataEnriched.tryEmit(it) }
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

    suspend fun fetchTextViaProxy(url: String, headers: Map<String, String> = emptyMap()): String =
        withContext(Dispatchers.IO) {
            val isLocal = url.startsWith("http://127.0.0.1:") || url.startsWith("http://localhost:")
            val targetUrl = if (isLocal) {
                url
            } else {
                preparePlaybackUrl(
                    StreamCandidate(
                        url = url,
                        kind = "auto",
                        label = "Subtitle",
                        headers = headers,
                        flags = listOf("requires_proxy"),
                    ),
                )
            }
            val conn = java.net.URL(targetUrl).openConnection() as java.net.HttpURLConnection
            conn.connectTimeout = 10_000
            conn.readTimeout = 15_000
            try {
                val code = conn.responseCode
                if (code !in 200..299) {
                    throw IllegalStateException("HTTP $code fetching subtitle")
                }
                conn.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
            } finally {
                conn.disconnect()
            }
        }

    suspend fun recordProgress(progress: PlaybackProgress) = withContext(Dispatchers.IO) {
        val withTimestamp = if (progress.lastWatchedAt.isBlank()) {
            progress.copy(lastWatchedAt = Instant.now().toString())
        } else {
            progress
        }
        requireCore().recordProgressJson(json.encodeToString(withTimestamp))
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

    suspend fun checkPluginUpdates(): List<PluginUpdateInfo> = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().checkPluginUpdatesJson())
    }

    suspend fun updatePlugin(pluginId: String): UpdatePluginResult = withContext(Dispatchers.IO) {
        val res = json.decodeFromString<UpdatePluginResult>(requireCore().updatePluginJson(pluginId))
        refreshCoreState()
        res
    }

    suspend fun updateAllPlugins(): BatchUpdateResult = withContext(Dispatchers.IO) {
        val res = json.decodeFromString<BatchUpdateResult>(requireCore().updateAllPluginsJson())
        refreshCoreState()
        res
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

    suspend fun setSimklClientId(clientId: String?) = withContext(Dispatchers.IO) {
        requireCore().setSimklClientId(clientId)
    }

    suspend fun getStreamsWithLinked(
        pluginId: String,
        nativeId: String,
        episodeId: String,
        season: Int,
        episode: Int,
        absoluteEpisode: Int? = null,
        isMovie: Boolean = false,
    ): StreamResult = withContext(Dispatchers.IO) {
        json.decodeFromString(
            requireCore().getStreamsWithLinkedJson(
                pluginId,
                nativeId,
                episodeId,
                season.toUInt(),
                episode.toUInt(),
                absoluteEpisode?.toUInt(),
                isMovie,
            ),
        )
    }

    suspend fun searchPluginSync(pluginId: String, query: String, page: Int = 1): SearchPageResult =
        withContext(Dispatchers.IO) {
            json.decodeFromString(requireCore().searchPluginSyncJson(pluginId, query, page.toUInt()))
        }

    suspend fun searchMetadata(
        query: String,
        provider: String = "all",
        preferredLang: String = _locale.value,
    ): MetadataSearchResponse = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().searchMetadataJson(query, provider, preferredLang))
    }

    suspend fun listSubtitlePlugins(): List<SubtitlePluginInfo> = withContext(Dispatchers.IO) {
        json.decodeFromString(requireCore().listSubtitlePluginsJson())
    }

    suspend fun searchSubtitles(
        query: SubtitleQueryPayload,
        targetPlugins: List<String>? = null,
    ): SubtitleSearchOutcome = withContext(Dispatchers.IO) {
        json.decodeFromString(
            requireCore().searchSubtitlesJson(json.encodeToString(query), targetPlugins),
        )
    }

    suspend fun resolveSubtitle(
        pluginId: String,
        subtitleId: String,
        lang: String,
        label: String,
    ): SubtitleTrack = withContext(Dispatchers.IO) {
        json.decodeFromString(
            requireCore().resolveSubtitleJson(pluginId, subtitleId, lang, label),
        )
    }

    suspend fun getLinkedSources(pluginId: String, nativeId: String): List<LinkedMediaSource> =
        withContext(Dispatchers.IO) {
            val c = core ?: return@withContext emptyList()
            json.decodeFromString(c.getLinkedSourcesJson(pluginId, nativeId))
        }

    suspend fun setLinkedSources(
        pluginId: String,
        nativeId: String,
        sources: List<LinkedMediaSource>,
    ): List<LinkedMediaSource> = withContext(Dispatchers.IO) {
        json.decodeFromString(
            requireCore().setLinkedSourcesJson(pluginId, nativeId, json.encodeToString(sources)),
        )
    }

    suspend fun saveUiPreferences(prefs: UiPreferences) = withContext(Dispatchers.IO) {
        val normalizedAspect = when (prefs.cards.aspectRatio) {
            "portrait_3x4" -> "portrait_3x4"
            "portrait_4x5" -> "portrait_4x5"
            "square" -> "square"
            "classic_4x3" -> "classic_4x3"
            "wide16x9", "wide_16x9" -> "wide_16x9"
            "cinema_21x9" -> "cinema_21x9"
            else -> "poster_2x3"
        }
        val cleanPrefs = prefs.copy(cards = prefs.cards.copy(aspectRatio = normalizedAspect))
        _uiPreferences.value = cleanPrefs
        runCatching {
            requireCore().setUiPreferencesJson(json.encodeToString(cleanPrefs))
        }.onFailure { err ->
            notifyError(err, "Failed to save preferences")
        }
    }

    suspend fun resetUiPreferences() = saveUiPreferences(UiPreferences())

    fun setAutoCheckUpdates(enabled: Boolean) {
        _autoCheckUpdates.value = enabled
        prefsStore.edit().putBoolean("auto_check_updates", enabled).apply()
    }

    fun skipUpdateVersion(version: String) {
        _skippedUpdateVersion.value = version
        prefsStore.edit().putString("skipped_update_version", version).apply()
        _showUpdateDialog.value = false
    }

    fun dismissUpdateDialog() {
        _showUpdateDialog.value = false
    }

    fun openUpdateDialog() {
        if (_updateInfo.value?.updateAvailable == true) {
            _showUpdateDialog.value = true
        }
    }

    suspend fun checkForAppUpdates(silent: Boolean = false): AppUpdateInfo? = withContext(Dispatchers.IO) {
        if (_checkingUpdate.value) return@withContext _updateInfo.value
        _checkingUpdate.value = true
        _updateError.value = null
        try {
            val conn = (URL("https://api.github.com/repos/falsisdev/vessel/releases?per_page=30")
                .openConnection() as HttpURLConnection).apply {
                connectTimeout = 10_000
                readTimeout = 15_000
                setRequestProperty("Accept", "application/vnd.github+json")
                setRequestProperty("X-GitHub-Api-Version", "2022-11-28")
                setRequestProperty("User-Agent", "Vessel-Android/${BuildConfig.VERSION_NAME}")
            }
            val bodyText = try {
                val code = conn.responseCode
                if (code !in 200..299) {
                    val errText = conn.errorStream?.bufferedReader()?.use { it.readText() }.orEmpty()
                    throw IllegalStateException("[VesselNetworkError]: GitHub releases returned HTTP $code ${errText.take(120)}")
                }
                conn.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
            } finally {
                conn.disconnect()
            }

            val arr = json.parseToJsonElement(bodyText).jsonArray
            val releases = arr.mapNotNull { runCatching { it.jsonObject }.getOrNull() }
                .filter { (it["draft"]?.jsonPrimitive?.booleanOrNull ?: false).not() }
            val chosen = releases.firstOrNull { (it["prerelease"]?.jsonPrimitive?.booleanOrNull ?: false).not() }
                ?: releases.firstOrNull()
                ?: throw IllegalStateException("[VesselNetworkError]: No GitHub releases found for falsisdev/vessel")

            val tagName = chosen["tag_name"]?.jsonPrimitive?.content ?: "v0.0.0"
            val latestVer = tagName.trimStart('v', 'V').ifBlank { "0.0.0" }
            val currentVer = BuildConfig.VERSION_NAME
            val updateAvailable = compareSemver(latestVer, currentVer) > 0

            val assets = chosen["assets"]?.jsonArray?.mapNotNull { runCatching { it.jsonObject }.getOrNull() }.orEmpty()
            val apkAsset = pickBestApkAsset(assets)

            val info = AppUpdateInfo(
                currentVersion = currentVer,
                latestVersion = latestVer,
                tagName = tagName,
                updateAvailable = updateAvailable,
                releaseName = chosen["name"]?.jsonPrimitive?.content?.takeIf { it.isNotBlank() } ?: "Vessel $tagName",
                releaseNotes = chosen["body"]?.jsonPrimitive?.content?.takeIf { it.isNotBlank() } ?: "No release notes provided.",
                publishedAt = chosen["published_at"]?.jsonPrimitive?.content,
                htmlUrl = chosen["html_url"]?.jsonPrimitive?.content ?: "https://github.com/falsisdev/vessel/releases",
                assetName = apkAsset?.get("name")?.jsonPrimitive?.content,
                assetSizeBytes = apkAsset?.get("size")?.jsonPrimitive?.longOrNull,
                downloadUrl = apkAsset?.get("browser_download_url")?.jsonPrimitive?.content,
            )
            _updateInfo.value = info
            if (updateAvailable && (!silent || _skippedUpdateVersion.value != info.latestVersion)) {
                _showUpdateDialog.value = true
            } else if (!silent && !updateAvailable) {
                notify("${VesselI18n.tr("settings.up_to_date")} (v$currentVer)")
            }
            info
        } catch (err: Throwable) {
            val msg = err.message ?: "[VesselNetworkError]: Failed to check updates"
            _updateError.value = msg
            if (!silent) notify(msg)
            null
        } finally {
            _checkingUpdate.value = false
        }
    }

    suspend fun downloadAndInstallUpdate(info: AppUpdateInfo) = withContext(Dispatchers.IO) {
        val urlStr = info.downloadUrl ?: return@withContext
        if (_downloadingUpdate.value) return@withContext

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !appContext.packageManager.canRequestPackageInstalls()) {
            notify(VesselI18n.tr("settings.install_permission_needed"))
            val permIntent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
                data = Uri.parse("package:${appContext.packageName}")
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            appContext.startActivity(permIntent)
            return@withContext
        }

        val parsed = Uri.parse(urlStr)
        val host = parsed.host?.lowercase().orEmpty()
        val trustedHost = parsed.scheme.equals("https", ignoreCase = true) &&
            (host == "github.com" || host.endsWith(".github.com") || host.endsWith(".githubusercontent.com"))
        if (!trustedHost) {
            val errMsg = "[VesselNetworkError]: Refusing update download from untrusted host $host"
            _updateError.value = errMsg
            notify(errMsg)
            return@withContext
        }

        _downloadingUpdate.value = true
        _updateDownloadPercent.value = 0
        _updateError.value = null
        try {
            val updatesDir = File(appContext.cacheDir, "updates").apply { mkdirs() }
            val safeName = (info.assetName ?: "vessel_${info.latestVersion}_android.apk")
                .replace(Regex("[^A-Za-z0-9._-]"), "_")
            val destFile = File(updatesDir, safeName)
            val tempFile = File(updatesDir, "$safeName.part")

            if (!(destFile.isFile && info.assetSizeBytes != null && destFile.length() == info.assetSizeBytes)) {
                val conn = (URL(urlStr).openConnection() as HttpURLConnection).apply {
                    instanceFollowRedirects = true
                    connectTimeout = 15_000
                    readTimeout = 30_000
                    setRequestProperty("User-Agent", "Vessel-Android/${BuildConfig.VERSION_NAME}")
                }
                try {
                    val code = conn.responseCode
                    if (code !in 200..299) {
                        throw IllegalStateException("[VesselNetworkError]: APK download failed with HTTP $code")
                    }
                    val total = info.assetSizeBytes ?: conn.contentLengthLong.takeIf { it > 0L }
                    var downloaded = 0L
                    conn.inputStream.use { input ->
                        FileOutputStream(tempFile).use { output ->
                            val buf = ByteArray(32 * 1024)
                            while (true) {
                                val read = input.read(buf)
                                if (read <= 0) break
                                output.write(buf, 0, read)
                                downloaded += read
                                _updateDownloadPercent.value = if (total != null && total > 0L) {
                                    ((downloaded * 100L) / total).toInt().coerceIn(0, 100)
                                } else null
                            }
                        }
                    }
                    if (destFile.exists()) destFile.delete()
                    tempFile.renameTo(destFile)
                } finally {
                    conn.disconnect()
                    if (tempFile.exists()) tempFile.delete()
                }
            } else {
                _updateDownloadPercent.value = 100
            }

            val apkUri = FileProvider.getUriForFile(appContext, "${appContext.packageName}.fileprovider", destFile)
            val installIntent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(apkUri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            appContext.startActivity(installIntent)
        } catch (err: Throwable) {
            val msg = err.message ?: "[VesselNetworkError]: Failed to download update APK"
            _updateError.value = msg
            notify(msg)
        } finally {
            _downloadingUpdate.value = false
        }
    }

    private fun pickBestApkAsset(assets: List<JsonObject>): JsonObject? {
        val apks = assets.filter {
            val name = it["name"]?.jsonPrimitive?.content?.lowercase().orEmpty()
            name.endsWith(".apk")
        }
        if (apks.isEmpty()) return null
        val supportedAbis = Build.SUPPORTED_ABIS.map { it.lowercase() }
        return apks.maxByOrNull { asset ->
            val n = asset["name"]?.jsonPrimitive?.content?.lowercase().orEmpty()
            var score = 10
            supportedAbis.forEachIndexed { idx, abi ->
                if (n.contains(abi)) score += (100 - idx * 10)
            }
            if (n.contains("universal") || n.contains("release")) score += 20
            if (n.contains("debug")) score -= 30
            score
        }
    }

    private fun compareSemver(a: String, b: String): Int {
        fun parse(raw: String): Pair<List<Int>, String?> {
            val trimmed = raw.trim().trimStart('v', 'V').substringBefore('+')
            val parts = trimmed.split('-', limit = 2)
            val nums = parts[0].split('.').map { it.toIntOrNull() ?: 0 }
            val pre = parts.getOrNull(1)?.takeIf { it.isNotBlank() }
            return listOf(
                nums.getOrElse(0) { 0 },
                nums.getOrElse(1) { 0 },
                nums.getOrElse(2) { 0 },
            ) to pre
        }
        val (na, pa) = parse(a)
        val (nb, pb) = parse(b)
        for (i in 0..2) {
            val c = na[i].compareTo(nb[i])
            if (c != 0) return c
        }
        return when {
            pa == null && pb == null -> 0
            pa == null -> 1
            pb == null -> -1
            else -> pa.compareTo(pb)
        }
    }

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
