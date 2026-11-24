package dev.falsis.vessel

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bookmarks
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.LiveTv
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.StreamCandidate
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.DomainPromptDialog
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselSplashOverlay
import dev.falsis.vessel.ui.components.VesselTextButton
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
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import dev.falsis.vessel.ui.theme.VesselTheme
import kotlinx.coroutines.delay
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
        val channelStreams: Map<String, List<StreamCandidate>> = emptyMap(),
    ) : OverlayRoute
}

class MainActivity : ComponentActivity() {
    private var deepLinkSource by mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            window.attributes = window.attributes.apply {
                layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            }
        }
        hideSystemBars()
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

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            hideSystemBars()
        }
    }

    private fun hideSystemBars() {
        val controller = WindowCompat.getInsetsController(window, window.decorView)
        controller.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        controller.hide(WindowInsetsCompat.Type.systemBars())
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
    val uriHandler = LocalUriHandler.current
    val theme by repo.activeTheme.collectAsState()
    val locale by repo.locale.collectAsState()
    val prompts by repo.domainPrompts.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()
    val isReady by repo.isReady.collectAsState()
    val initError by repo.initError.collectAsState()
    val autoCheckUpdates by repo.autoCheckUpdates.collectAsState()
    val showUpdateDialog by repo.showUpdateDialog.collectAsState()
    val updateInfo by repo.updateInfo.collectAsState()
    val downloadingUpdate by repo.downloadingUpdate.collectAsState()
    val updatePercent by repo.updateDownloadPercent.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(isReady, autoCheckUpdates) {
        if (isReady && autoCheckUpdates && repo.updateInfo.value == null) {
            delay(1800)
            repo.checkForAppUpdates(silent = true)
        }
    }

    var selectedTab by remember { mutableIntStateOf(0) }
    var overlayStack by remember { mutableStateOf<List<OverlayRoute>>(emptyList()) }
    var splashAnimDone by remember { mutableStateOf(false) }
    val showSplash = !splashAnimDone || (!isReady && initError == null)

    val overlay = overlayStack.lastOrNull()

    fun pushOverlay(route: OverlayRoute) {
        overlayStack = overlayStack + route
    }

    fun popOverlay() {
        if (overlayStack.isNotEmpty()) {
            overlayStack = overlayStack.dropLast(1)
        }
    }

    BackHandler(enabled = overlayStack.isNotEmpty()) {
        popOverlay()
    }

    LaunchedEffect(deepLinkSource) {
        if (!deepLinkSource.isNullOrBlank()) {
            pushOverlay(OverlayRoute.Extensions(deepLinkSource))
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
                Scaffold(
                    snackbarHost = { SnackbarHost(snackbarHostState) },
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                ) { innerPadding ->
                    val contentModifier = if (overlay is OverlayRoute.Player) {
                        Modifier.fillMaxSize()
                    } else {
                        Modifier.fillMaxSize().padding(innerPadding)
                    }
                    Box(modifier = contentModifier) {
                        AnimatedContent(
                            targetState = overlay,
                            transitionSpec = {
                                val easing = FastOutSlowInEasing
                                when {
                                    targetState is OverlayRoute.Player -> {
                                        (fadeIn(tween(260, easing = easing)) + scaleIn(initialScale = 0.96f, animationSpec = tween(260, easing = easing)))
                                            .togetherWith(fadeOut(tween(200)))
                                    }
                                    initialState is OverlayRoute.Player -> {
                                        fadeIn(tween(240, easing = easing))
                                            .togetherWith(fadeOut(tween(220)) + scaleOut(targetScale = 0.97f, animationSpec = tween(220)))
                                    }
                                    targetState != null -> {
                                        (slideInHorizontally(tween(260, easing = easing)) { (it * 0.28f).toInt() } + fadeIn(tween(240)))
                                            .togetherWith(slideOutHorizontally(tween(220, easing = easing)) { -(it * 0.14f).toInt() } + fadeOut(tween(180)))
                                    }
                                    else -> {
                                        (slideInHorizontally(tween(240, easing = easing)) { -(it * 0.14f).toInt() } + fadeIn(tween(220)))
                                            .togetherWith(slideOutHorizontally(tween(240, easing = easing)) { (it * 0.28f).toInt() } + fadeOut(tween(200)))
                                    }
                                }
                            },
                            label = "overlay_nav_transition",
                        ) { activeOverlay ->
                            when (activeOverlay) {
                                is OverlayRoute.Player -> PlayerScreen(
                                    repo = repo,
                                    pluginId = activeOverlay.pluginId,
                                    nativeId = activeOverlay.nativeId,
                                    detail = activeOverlay.detail,
                                    episode = activeOverlay.episode,
                                    streamResult = activeOverlay.streamResult,
                                    initialStreamIndex = activeOverlay.streamIndex,
                                    initialPositionMs = activeOverlay.positionMs,
                                    channelStreams = activeOverlay.channelStreams,
                                    onBack = { popOverlay() },
                                    onGoToDetails = {
                                        val detailsRoute = OverlayRoute.Details(activeOverlay.pluginId, activeOverlay.nativeId)
                                        val prevIsSameDetails = overlayStack.dropLast(1).lastOrNull() == detailsRoute
                                        if (prevIsSameDetails) {
                                            popOverlay()
                                        } else {
                                            overlayStack = overlayStack.dropLast(1) + detailsRoute
                                        }
                                    },
                                )
                                is OverlayRoute.Details -> DetailsScreen(
                                    repo = repo,
                                    pluginId = activeOverlay.pluginId,
                                    nativeId = activeOverlay.nativeId,
                                    onBack = { popOverlay() },
                                    onPlayStream = { detail, ep, streams, idx, posMs ->
                                        pushOverlay(
                                            OverlayRoute.Player(
                                                pluginId = activeOverlay.pluginId,
                                                nativeId = activeOverlay.nativeId,
                                                detail = detail,
                                                episode = ep,
                                                streamResult = streams,
                                                streamIndex = idx,
                                                positionMs = posMs,
                                            ),
                                        )
                                    },
                                    onPlayLiveReplace = { detail, ep, streams ->
                                        overlayStack = overlayStack.dropLast(1) + OverlayRoute.Player(
                                            pluginId = activeOverlay.pluginId,
                                            nativeId = activeOverlay.nativeId,
                                            detail = detail,
                                            episode = ep,
                                            streamResult = streams,
                                            streamIndex = 0,
                                            positionMs = 0L,
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
                                        pushOverlay(
                                            OverlayRoute.Player(dl.pluginId, dl.nativeId, offDetail, offEp, offStreams, 0, 0L),
                                        )
                                    },
                                    onBack = { popOverlay() },
                                )
                                is OverlayRoute.Extensions -> ExtensionsScreen(
                                    repo = repo,
                                    initialSourceInput = activeOverlay.initialSource,
                                    onBack = { popOverlay() },
                                )
                                null -> {
                                    val palette = LocalVesselPalette.current
                                    val tabBody: @Composable (Modifier) -> Unit = { bodyMod ->
                                        Box(modifier = bodyMod.fillMaxSize()) {
                                            Crossfade(
                                                targetState = selectedTab,
                                                animationSpec = tween(durationMillis = 200, easing = FastOutSlowInEasing),
                                                label = "top_tab_crossfade",
                                            ) { tabIdx ->
                                                when (TopTab.entries[tabIdx]) {
                                                    TopTab.HOME -> HomeScreen(
                                                        repo = repo,
                                                        onOpenDetails = { pid, nid -> pushOverlay(OverlayRoute.Details(pid, nid)) },
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
                                                                    pushOverlay(
                                                                        OverlayRoute.Player(
                                                                            pluginId = entry.progress.media.pluginId,
                                                                            nativeId = entry.progress.media.nativeId,
                                                                            detail = detail,
                                                                            episode = ep,
                                                                            streamResult = sr,
                                                                            streamIndex = 0,
                                                                            positionMs = if (entry.progress.completed) 0L else entry.progress.positionMs,
                                                                        ),
                                                                    )
                                                                }.onFailure {
                                                                    pushOverlay(OverlayRoute.Details(entry.progress.media.pluginId, entry.progress.media.nativeId))
                                                                }
                                                            }
                                                        },
                                                        onPlayHero = { pid, nid, detail, ep, sr, idx, posMs ->
                                                            pushOverlay(
                                                                OverlayRoute.Player(
                                                                    pluginId = pid,
                                                                    nativeId = nid,
                                                                    detail = detail,
                                                                    episode = ep,
                                                                    streamResult = sr,
                                                                    streamIndex = idx,
                                                                    positionMs = posMs,
                                                                ),
                                                            )
                                                        },
                                                        onOpenExtensions = { pushOverlay(OverlayRoute.Extensions(null)) },
                                                    )
                                                    TopTab.SEARCH -> SearchScreen(
                                                        repo = repo,
                                                        onOpenDetails = { pid, nid -> pushOverlay(OverlayRoute.Details(pid, nid)) },
                                                        onPlayLiveStream = { pid, detail, ep, sr ->
                                                            pushOverlay(OverlayRoute.Player(pid, detail.nativeId, detail, ep, sr, 0, 0L))
                                                        },
                                                    )
                                                    TopTab.LIBRARY -> LibraryScreen(
                                                        repo = repo,
                                                        onOpenDetails = { pid, nid -> pushOverlay(OverlayRoute.Details(pid, nid)) },
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
                                                            pushOverlay(OverlayRoute.Player("iptv", url, liveDetail, liveEp, liveStreams, 0, 0L))
                                                        },
                                                        onPlayLiveStream = { pid, detail, ep, sr, channelStreams ->
                                                            pushOverlay(OverlayRoute.Player(pid, detail.nativeId, detail, ep, sr, 0, 0L, channelStreams))
                                                        },
                                                        onOpenDetails = { pid, nid -> pushOverlay(OverlayRoute.Details(pid, nid)) },
                                                    )
                                                    TopTab.SETTINGS -> SettingsScreen(
                                                        repo = repo,
                                                        onOpenDownloads = { pushOverlay(OverlayRoute.Downloads) },
                                                        onOpenExtensions = { pushOverlay(OverlayRoute.Extensions(null)) },
                                                    )
                                                }
                                            }
                                        }
                                    }

                                    if (prefs.shell.mobileNavLayout == "floating_bottom") {
                                        Scaffold(
                                            containerColor = palette.bgBase,
                                            contentWindowInsets = WindowInsets(0, 0, 0, 0),
                                            bottomBar = {
                                                Box(
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .navigationBarsPadding()
                                                        .padding(horizontal = 14.dp, vertical = 10.dp),
                                                ) {
                                                    Surface(
                                                        shape = RoundedCornerShape(999.dp),
                                                        color = palette.bgSurface.copy(alpha = 0.94f),
                                                        border = BorderStroke(1.dp, palette.border.copy(alpha = 0.65f)),
                                                        shadowElevation = 14.dp,
                                                        modifier = Modifier.fillMaxWidth(),
                                                    ) {
                                                        Row(
                                                            modifier = Modifier
                                                                .fillMaxWidth()
                                                                .padding(horizontal = 6.dp, vertical = 5.dp),
                                                            horizontalArrangement = Arrangement.SpaceEvenly,
                                                        ) {
                                                            TopTab.entries.forEachIndexed { idx, tab ->
                                                                val isSelected = selectedTab == idx
                                                                val pillColor by animateColorAsState(
                                                                    targetValue = if (isSelected) palette.accent.copy(alpha = 0.20f) else palette.bgSurface.copy(alpha = 0f),
                                                                    animationSpec = tween(200),
                                                                    label = "float_nav_bg",
                                                                )
                                                                val contentTint by animateColorAsState(
                                                                    targetValue = if (isSelected) palette.accent else palette.textMuted,
                                                                    animationSpec = tween(200),
                                                                    label = "float_nav_tint",
                                                                )
                                                                Column(
                                                                    horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
                                                                    verticalArrangement = Arrangement.Center,
                                                                    modifier = Modifier
                                                                        .weight(1f)
                                                                        .clip(RoundedCornerShape(999.dp))
                                                                        .background(pillColor)
                                                                        .clickable { selectedTab = idx }
                                                                        .padding(horizontal = 4.dp, vertical = 6.dp),
                                                                ) {
                                                                    Icon(
                                                                        imageVector = tab.icon,
                                                                        contentDescription = VesselI18n.tr(tab.key),
                                                                        tint = contentTint,
                                                                        modifier = Modifier.size(19.dp),
                                                                    )
                                                                    Text(
                                                                        text = VesselI18n.tr(tab.key),
                                                                        color = contentTint,
                                                                        maxLines = 1,
                                                                        softWrap = false,
                                                                        overflow = TextOverflow.Ellipsis,
                                                                        textAlign = TextAlign.Center,
                                                                        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
                                                                        style = MaterialTheme.typography.labelSmall,
                                                                    )
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                        ) { navInnerPadding ->
                                            tabBody(Modifier.padding(bottom = navInnerPadding.calculateBottomPadding()))
                                        }
                                    } else {
                                        NavigationSuiteScaffold(
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
                                            tabBody(Modifier)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if (showSplash) {
                    VesselSplashOverlay(onDone = { splashAnimDone = true })
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

            val pendingUpdate = updateInfo
            if (showUpdateDialog && pendingUpdate != null && pendingUpdate.updateAvailable) {
                val palette = LocalVesselPalette.current
                AlertDialog(
                    onDismissRequest = { repo.dismissUpdateDialog() },
                    title = {
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(
                                text = VesselI18n.tr("settings.update_available"),
                                style = MaterialTheme.typography.labelMedium,
                                color = palette.accent,
                                fontWeight = FontWeight.Bold,
                            )
                            Text(
                                text = pendingUpdate.releaseName,
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                    },
                    text = {
                        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text(
                                text = "v${pendingUpdate.currentVersion} → v${pendingUpdate.latestVersion}",
                                style = MaterialTheme.typography.labelLarge,
                                color = palette.textPrimary,
                                fontWeight = FontWeight.SemiBold,
                            )
                            Surface(
                                shape = RoundedCornerShape(10.dp),
                                color = palette.bgSurface,
                            ) {
                                Column(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .heightIn(max = 210.dp)
                                        .verticalScroll(rememberScrollState())
                                        .padding(10.dp),
                                ) {
                                    Text(
                                        text = pendingUpdate.releaseNotes,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = palette.textMuted,
                                    )
                                }
                            }
                            if (downloadingUpdate) {
                                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                    Text(
                                        text = VesselI18n.trf(
                                            "settings.downloading_update",
                                            "file" to (pendingUpdate.assetName ?: "vessel.apk"),
                                            "percent" to (updatePercent ?: 0),
                                        ),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                    )
                                    if (updatePercent != null) {
                                        LinearProgressIndicator(
                                            progress = { (updatePercent ?: 0) / 100f },
                                            modifier = Modifier.fillMaxWidth().height(5.dp).clip(RoundedCornerShape(999.dp)),
                                        )
                                    } else {
                                        LinearProgressIndicator(
                                            modifier = Modifier.fillMaxWidth().height(5.dp).clip(RoundedCornerShape(999.dp)),
                                        )
                                    }
                                }
                            } else if (pendingUpdate.downloadUrl == null) {
                                Text(
                                    text = VesselI18n.tr("settings.update_asset_pending"),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = palette.textMuted,
                                )
                            }
                        }
                    },
                    confirmButton = {
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            VesselOutlinedButton(
                                height = 32.dp,
                                onClick = { runCatching { uriHandler.openUri(pendingUpdate.htmlUrl) } },
                            ) {
                                Text(VesselI18n.tr("settings.open_releases"), style = MaterialTheme.typography.labelSmall)
                            }
                            if (pendingUpdate.downloadUrl != null) {
                                VesselButton(
                                    height = 32.dp,
                                    enabled = !downloadingUpdate,
                                    onClick = { scope.launch { repo.downloadAndInstallUpdate(pendingUpdate) } },
                                ) {
                                    Text(
                                        text = if (downloadingUpdate) {
                                            VesselI18n.tr("settings.downloading_short")
                                        } else {
                                            VesselI18n.trf("settings.install_update", "asset" to (pendingUpdate.assetName ?: "APK"))
                                        },
                                        style = MaterialTheme.typography.labelSmall,
                                    )
                                }
                            }
                        }
                    },
                    dismissButton = {
                        VesselTextButton(onClick = { repo.skipUpdateVersion(pendingUpdate.latestVersion) }) {
                            Text(VesselI18n.trf("settings.skip_version", "version" to "v${pendingUpdate.latestVersion}"))
                        }
                    },
                )
            }
        }
    }
}
