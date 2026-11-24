package dev.falsis.vessel

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bookmarks
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.LiveTv
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.LayoutDirection
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.DomainPromptDialog
import dev.falsis.vessel.ui.components.VesselSplashOverlay
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.screens.DetailsScreen
import dev.falsis.vessel.ui.screens.DownloadsScreen
import dev.falsis.vessel.ui.screens.ExtensionsScreen
import dev.falsis.vessel.ui.screens.HomeScreen
import dev.falsis.vessel.ui.screens.LibraryScreen
import dev.falsis.vessel.ui.screens.LiveTvScreen
import dev.falsis.vessel.ui.screens.PlayerScreen
import dev.falsis.vessel.ui.screens.SearchScreen
import dev.falsis.vessel.ui.screens.SettingsScreen
import dev.falsis.vessel.ui.theme.VesselTheme
import kotlinx.coroutines.launch

enum class TopTab(val key: String, val icon: ImageVector) {
    HOME("nav.home", Icons.Default.Home),
    SEARCH("nav.search", Icons.Default.Search),
    LIBRARY("nav.library", Icons.Default.Bookmarks),
    LIVE_TV("nav.live_tv", Icons.Default.LiveTv),
    SETTINGS("nav.settings", Icons.Default.Settings),
}

sealed interface OverlayRoute {
    data object Downloads : OverlayRoute
    data class Extensions(val initialSource: String? = null) : OverlayRoute
    data class Details(val pluginId: String, val nativeId: String) : OverlayRoute
    data class Player(
        val pluginId: String,
        val nativeId: String,
        val detail: MediaDetail,
        val episode: Episode,
        val streamResult: StreamResult,
        val streamIndex: Int,
        val positionMs: Long,
    ) : OverlayRoute
}

class MainActivity : ComponentActivity() {
    private var deepLinkSource by mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        handleIntent(intent)

        val repo = VesselRepository.get(this)
        setContent {
            VesselAndroidApp(
                repo = repo,
                deepLinkSource = deepLinkSource,
                onClearDeepLink = { deepLinkSource = null },
            )
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        val uri = intent?.data ?: return
        if (uri.scheme == "vessel" && uri.host == "install") {
            deepLinkSource = uri.getQueryParameter("src") ?: uri.toString()
        }
    }
}

@Composable
fun VesselAndroidApp(
    repo: VesselRepository,
    deepLinkSource: String?,
    onClearDeepLink: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val theme by repo.activeTheme.collectAsState()
    val locale by repo.locale.collectAsState()
    val prompts by repo.domainPrompts.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    var selectedTab by remember { mutableIntStateOf(0) }
    var overlay by remember { mutableStateOf<OverlayRoute?>(null) }
    var showSplash by remember { mutableStateOf(true) }

    BackHandler(enabled = overlay != null) {
        overlay = null
    }

    LaunchedEffect(deepLinkSource) {
        if (!deepLinkSource.isNullOrBlank()) {
            overlay = OverlayRoute.Extensions(deepLinkSource)
            onClearDeepLink()
        }
    }

    LaunchedEffect(Unit) {
        repo.toasts.collect { msg -> snackbarHostState.showSnackbar(msg) }
    }

    val dir = if (VesselI18n.isRtl(locale)) LayoutDirection.Rtl else LayoutDirection.Ltr

    VesselTheme(themeSpec = theme) {
        CompositionLocalProvider(LocalLayoutDirection provides dir) {
            Box(modifier = Modifier.fillMaxSize()) {
                Scaffold(snackbarHost = { SnackbarHost(snackbarHostState) }) { innerPadding ->
                    Box(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
                        when (val activeOverlay = overlay) {
                            is OverlayRoute.Player -> PlayerScreen(
                                repo = repo,
                                pluginId = activeOverlay.pluginId,
                                nativeId = activeOverlay.nativeId,
                                detail = activeOverlay.detail,
                                episode = activeOverlay.episode,
                                streamResult = activeOverlay.streamResult,
                                initialStreamIndex = activeOverlay.streamIndex,
                                initialPositionMs = activeOverlay.positionMs,
                                onBack = { overlay = null },
                                onGoToDetails = {
                                    overlay = OverlayRoute.Details(activeOverlay.pluginId, activeOverlay.nativeId)
                                },
                            )
                            is OverlayRoute.Details -> DetailsScreen(
                                repo = repo,
                                pluginId = activeOverlay.pluginId,
                                nativeId = activeOverlay.nativeId,
                                onBack = { overlay = null },
                                onPlayStream = { detail, ep, streams, idx, posMs ->
                                    overlay = OverlayRoute.Player(
                                        pluginId = activeOverlay.pluginId,
                                        nativeId = activeOverlay.nativeId,
                                        detail = detail,
                                        episode = ep,
                                        streamResult = streams,
                                        streamIndex = idx,
                                        positionMs = posMs,
                                    )
                                },
                            )
                            is OverlayRoute.Downloads -> DownloadsScreen(
                                repo = repo,
                                onPlayOffline = { dl, bundle ->
                                    val offDetail = MediaDetail(
                                        nativeId = dl.nativeId,
                                        type = if (dl.season == null) "movie" else "series",
                                        title = dl.title,
                                    )
                                    val offEp = Episode(
                                        streamId = dl.id,
                                        season = dl.season ?: 0,
                                        episode = dl.episode ?: 0,
                                        title = dl.streamLabel,
                                    )
                                    val offStreams = StreamResult(
                                        streams = listOf(
                                            StreamCandidate(
                                                url = bundle.url,
                                                kind = bundle.kind,
                                                label = dl.streamLabel ?: dl.title,
                                                isDefault = true,
                                                subtitles = bundle.subtitles,
                                            ),
                                        ),
                                    )
                                    overlay = OverlayRoute.Player(dl.pluginId, dl.nativeId, offDetail, offEp, offStreams, 0, 0L)
                                },
                                onBack = { overlay = null },
                            )
                            is OverlayRoute.Extensions -> ExtensionsScreen(
                                repo = repo,
                                initialSourceInput = activeOverlay.initialSource,
                                onBack = { overlay = null },
                            )
                            null -> NavigationSuiteScaffold(
                                navigationSuiteItems = {
                                    TopTab.entries.forEachIndexed { idx, tab ->
                                        item(
                                            selected = selectedTab == idx,
                                            onClick = { selectedTab = idx },
                                            icon = { Icon(tab.icon, contentDescription = VesselI18n.tr(tab.key)) },
                                            label = {
                                                Text(
                                                    text = VesselI18n.tr(tab.key),
                                                    maxLines = 1,
                                                    softWrap = false,
                                                    overflow = TextOverflow.Ellipsis,
                                                    textAlign = TextAlign.Center,
                                                    style = MaterialTheme.typography.labelSmall,
                                                )
                                            },
                                        )
                                    }
                                },
                            ) {
                                when (TopTab.entries[selectedTab]) {
                                    TopTab.HOME -> HomeScreen(
                                        repo = repo,
                                        onOpenDetails = { pid, nid -> overlay = OverlayRoute.Details(pid, nid) },
                                        onResumeEntry = { entry ->
                                            scope.launch {
                                                runCatching {
                                                    val detail = repo.getDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                                    val sr = repo.getStreams(entry.progress.media.pluginId, entry.progress.streamId)
                                                    val ep = detail.episodes.firstOrNull { it.streamId == entry.progress.streamId }
                                                        ?: Episode(
                                                            streamId = entry.progress.streamId,
                                                            season = entry.progress.season,
                                                            episode = entry.progress.episode,
                                                            title = entry.progress.episodeTitle,
                                                        )
                                                    overlay = OverlayRoute.Player(
                                                        pluginId = entry.progress.media.pluginId,
                                                        nativeId = entry.progress.media.nativeId,
                                                        detail = detail,
                                                        episode = ep,
                                                        streamResult = sr,
                                                        streamIndex = 0,
                                                        positionMs = if (entry.progress.completed) 0L else entry.progress.positionMs,
                                                    )
                                                }.onFailure {
                                                    overlay = OverlayRoute.Details(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                                }
                                            }
                                        },
                                        onOpenExtensions = { overlay = OverlayRoute.Extensions(null) },
                                    )
                                    TopTab.SEARCH -> SearchScreen(
                                        repo = repo,
                                        onOpenDetails = { pid, nid -> overlay = OverlayRoute.Details(pid, nid) },
                                    )
                                    TopTab.LIBRARY -> LibraryScreen(
                                        repo = repo,
                                        onOpenDetails = { pid, nid -> overlay = OverlayRoute.Details(pid, nid) },
                                    )
                                    TopTab.LIVE_TV -> LiveTvScreen(
                                        repo = repo,
                                        onPlayUrl = { title, url ->
                                            val liveDetail = MediaDetail(
                                                nativeId = url,
                                                type = "live",
                                                title = title,
                                            )
                                            val liveEp = Episode(streamId = url, season = 0, episode = 0, title = title)
                                            val liveStreams = StreamResult(
                                                streams = listOf(StreamCandidate(url = url, kind = "auto", label = title, isDefault = true)),
                                            )
                                            overlay = OverlayRoute.Player("iptv", url, liveDetail, liveEp, liveStreams, 0, 0L)
                                        },
                                        onOpenDetails = { pid, nid -> overlay = OverlayRoute.Details(pid, nid) },
                                    )
                                    TopTab.SETTINGS -> SettingsScreen(
                                        repo = repo,
                                        onOpenDownloads = { overlay = OverlayRoute.Downloads },
                                        onOpenExtensions = { overlay = OverlayRoute.Extensions(null) },
                                    )
                                }
                            }
                        }
                    }
                }

                if (showSplash) {
                    VesselSplashOverlay(onDone = { showSplash = false })
                }
            }

            prompts.firstOrNull()?.let { prompt ->
                DomainPromptDialog(
                    prompt = prompt,
                    onRespond = { allow, permanent ->
                        scope.launch { repo.respondDomainPrompt(prompt, allow, permanent) }
                    },
                )
            }
        }
    }
}
