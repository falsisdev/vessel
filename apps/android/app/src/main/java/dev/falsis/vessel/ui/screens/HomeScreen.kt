package dev.falsis.vessel.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsIgnoringVisibility
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DragIndicator
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import dev.falsis.vessel.data.PlaybackProgress
import dev.falsis.vessel.ui.theme.LocalIsTvDevice
import dev.falsis.vessel.ui.theme.LocalVesselFloatingNavInset
import dev.falsis.vessel.ui.theme.LocalVesselUiTokens
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.ContinueWatchingEntry
import dev.falsis.vessel.data.Episode
import dev.falsis.vessel.data.HomeShelfResult
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.StreamResult
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.ContinueWatchingCard
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.components.VesselButton
import dev.falsis.vessel.ui.components.VesselCompactIconButton
import dev.falsis.vessel.ui.components.VesselFilterChip
import dev.falsis.vessel.ui.components.VesselLogoMark
import dev.falsis.vessel.ui.components.VesselOutlinedButton
import dev.falsis.vessel.ui.components.VesselOutlinedTextField
import dev.falsis.vessel.ui.components.VesselTextButton
import dev.falsis.vessel.ui.components.vesselDpadInputEscape
import dev.falsis.vessel.ui.components.vesselFocusBorder
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.isLiveTvPlugin
import dev.falsis.vessel.ui.resolveLiveGroupName
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import java.util.Locale
import kotlin.math.abs
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import android.util.Log

private data class SpotlightHeroItem(
    val summary: MediaSummary,
    val pluginId: String,
    val pluginName: String,
)

private data class HeroSourcePromptState(
    val pluginId: String,
    val nativeId: String,
    val detail: MediaDetail,
    val episode: Episode,
    val streamResult: StreamResult,
    val resumePosMs: Long,
)

private fun pickDefaultStarterPlugins(homeEligible: List<dev.falsis.vessel.data.PluginListEntry>): List<String> {
    // Same smart curated <= 5 VOD/anime starter selection as Desktop HomeView.vue
    val preferred = listOf("anizium", "diziwatch", "yabancidizi", "animecix", "seicode")
    val vodIds = homeEligible.map { it.manifest.id }.toSet()
    val picked = mutableListOf<String>()
    for (id in preferred) {
        if (id in vodIds && picked.size < 5) picked += id
    }
    for (p in homeEligible) {
        if (p.manifest.id !in picked && picked.size < 5) picked += p.manifest.id
    }
    return picked
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun HomeScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
    onResumeEntry: (ContinueWatchingEntry) -> Unit,
    onPlayHero: (
        pluginId: String,
        nativeId: String,
        detail: MediaDetail,
        episode: Episode,
        streamResult: StreamResult,
        streamIndex: Int,
        startPosMs: Long,
    ) -> Unit,
    onOpenExtensions: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val isReady by repo.isReady.collectAsState()
    val initError by repo.initError.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()

    var loading by remember { mutableStateOf(true) }
    var launchingHeroKey by remember { mutableStateOf<String?>(null) }
    var shelves by remember { mutableStateOf<List<HomeShelfResult>>(emptyList()) }
    var continueWatching by remember { mutableStateOf<List<ContinueWatchingEntry>>(emptyList()) }
    var showFeedsModal by remember { mutableStateOf(false) }
    var editingCwEntry by remember { mutableStateOf<ContinueWatchingEntry?>(null) }
    var heroSourceModal by remember {
        mutableStateOf<HeroSourcePromptState?>(null)
    }
    val heroDetails = remember { mutableStateMapOf<String, MediaDetail>() }

    fun watchHeroNow(pluginId: String, nativeId: String) {
        val key = "$pluginId:$nativeId"
        if (launchingHeroKey != null) return
        launchingHeroKey = key
        scope.launch {
            runCatching {
                val loadedDetail = heroDetails[key] ?: repo.getDetails(pluginId, nativeId).also {
                    heroDetails[key] = it
                }
                val cwEntry = continueWatching.firstOrNull {
                    it.progress.media.pluginId == pluginId && it.progress.media.nativeId == nativeId
                }
                val progRows = runCatching { repo.listMediaWatchProgress(pluginId, nativeId) }.getOrDefault(emptyList())
                val latestInProgress = progRows.filter { !it.completed }.maxByOrNull { it.lastWatchedAt }
                val targetEp = cwEntry?.let { cw ->
                    loadedDetail.episodes.firstOrNull { it.streamId == cw.progress.streamId }
                        ?: loadedDetail.episodes.firstOrNull {
                            it.season == cw.progress.season && it.episode == cw.progress.episode
                        }
                } ?: latestInProgress?.let { p ->
                    loadedDetail.episodes.firstOrNull { it.streamId == p.streamId }
                        ?: loadedDetail.episodes.firstOrNull { it.season == p.season && it.episode == p.episode }
                } ?: loadedDetail.episodes.firstOrNull()
                    ?: if (loadedDetail.type == "movie") {
                        Episode(
                            streamId = loadedDetail.nativeId,
                            season = 1,
                            episode = 1,
                            title = loadedDetail.title,
                        )
                    } else {
                        null
                    }

                if (targetEp == null) {
                    repo.notify(VesselI18n.tr("details.no_episodes"))
                    return@runCatching
                }
                val sr = repo.getStreamsWithLinked(
                    pluginId = pluginId,
                    nativeId = nativeId,
                    episodeId = targetEp.streamId,
                    season = targetEp.season,
                    episode = targetEp.episode,
                    absoluteEpisode = targetEp.absoluteEpisode,
                    isMovie = loadedDetail.type == "movie",
                )
                if (sr.streams.isEmpty()) {
                    repo.notify(VesselI18n.tr("details.no_streams"))
                    return@runCatching
                }
                val resumePos = when {
                    cwEntry != null && !cwEntry.progress.completed && cwEntry.progress.streamId == targetEp.streamId ->
                        cwEntry.progress.positionMs
                    latestInProgress != null && latestInProgress.streamId == targetEp.streamId ->
                        latestInProgress.positionMs
                    else -> 0L
                }
                val defaultPlayable = sr.streams.indexOfFirst { it.isDefault && !it.needsExtractor && it.kind != "embed" }
                val firstPlayable = sr.streams.indexOfFirst { !it.needsExtractor && it.kind != "embed" }
                val bestIdx = if (defaultPlayable >= 0) defaultPlayable else firstPlayable
                val askAlways = prefs.playback.clickBehavior == "always_ask_source" && sr.streams.size > 1
                if (askAlways || bestIdx < 0) {
                    heroSourceModal = HeroSourcePromptState(
                        pluginId = pluginId,
                        nativeId = nativeId,
                        detail = loadedDetail,
                        episode = targetEp,
                        streamResult = sr,
                        resumePosMs = resumePos,
                    )
                } else {
                    onPlayHero(pluginId, nativeId, loadedDetail, targetEp, sr, bestIdx, resumePos)
                }
            }.onFailure {
                repo.notifyError(it, "Failed to start playback")
            }
            launchingHeroKey = null
        }
    }

    val enabledPlugins = remember(plugins) { plugins.filter { it.enabled } }
    val livePluginIds = remember(plugins) { plugins.filter(::isLiveTvPlugin).map { it.manifest.id }.toSet() }
    val homeEligiblePlugins = remember(enabledPlugins) { enabledPlugins.filterNot(::isLiveTvPlugin) }
    val homeEligibleIds = remember(homeEligiblePlugins) { homeEligiblePlugins.map { it.manifest.id } }
    val pluginNameById = remember(plugins) {
        plugins.associate { it.manifest.id to it.manifest.name }
    }
    val effectiveHomeIds = remember(homeEligiblePlugins, prefs.home.selectedPlugins) {
        val eligibleSet = homeEligiblePlugins.map { it.manifest.id }.toSet()
        val explicit = prefs.home.selectedPlugins
        when {
            explicit == null -> pickDefaultStarterPlugins(homeEligiblePlugins)
            explicit.isEmpty() -> emptyList()
            else -> explicit.filter { it in eligibleSet }
        }
    }
    val effectiveHeroIds = remember(homeEligibleIds, prefs.home.heroPlugins) {
        val eligibleSet = homeEligibleIds.toSet()
        prefs.home.heroPlugins?.filter { it in eligibleSet }.orEmpty()
    }
    val orderedShelves = remember(shelves, effectiveHomeIds, livePluginIds) {
        val allowed = effectiveHomeIds.toSet()
        shelves
            .filter { it.pluginId in allowed && it.pluginId !in livePluginIds }
            .map { shelf ->
                shelf.copy(
                    sections = shelf.sections
                        .map { sec -> sec.copy(items = sec.items.filterNot { it.type == "live" }) }
                        .filter { it.items.isNotEmpty() },
                )
            }
            .sortedBy {
                val idx = effectiveHomeIds.indexOf(it.pluginId)
                if (idx == -1) Int.MAX_VALUE else idx
            }
    }

    suspend fun loadHome(forceRefresh: Boolean = false) {
        if (initError != null) {
            loading = false
            return
        }
        loading = true
        if (forceRefresh) {
            runCatching { repo.clearRuntimeCaches() }
        }
        runCatching { continueWatching = repo.listContinueWatching(prefs.home.continueWatchingLimit) }
            .onFailure { Log.e("VesselHome", "listContinueWatching failed", it) }
        val targetFeedIds = (effectiveHomeIds + effectiveHeroIds).distinct()
        runCatching {
            shelves = if (targetFeedIds.isEmpty()) emptyList() else repo.getHomeFeeds(targetFeedIds)
        }.onFailure { Log.e("VesselHome", "getHomeFeeds failed", it) }
        loading = false
    }

    LaunchedEffect(isReady, effectiveHomeIds, effectiveHeroIds) {
        if (isReady) loadHome()
    }

    val spotlightPool: List<SpotlightHeroItem> = remember(shelves, orderedShelves, effectiveHeroIds, livePluginIds) {
        val heroFilter = effectiveHeroIds.toSet()
        val nonLiveShelves = shelves.filter { it.pluginId !in livePluginIds }
        val filteredShelves = if (heroFilter.isNotEmpty()) {
            val matched = nonLiveShelves.filter { it.pluginId in heroFilter }
            if (matched.isNotEmpty()) matched else orderedShelves
        } else {
            orderedShelves.ifEmpty { nonLiveShelves }
        }
        val allCandidates = filteredShelves.flatMap { shelf ->
            shelf.sections.flatMap { sec ->
                sec.items.mapNotNull { item ->
                    val pid = item.pluginId ?: shelf.pluginId
                    if (item.type == "live" || pid in livePluginIds) null
                    else SpotlightHeroItem(
                        summary = item,
                        pluginId = pid,
                        pluginName = shelf.pluginName,
                    )
                }
            }
        }
        val withPoster = allCandidates.filter { !it.summary.poster.isNullOrBlank() }
        val pool = withPoster.ifEmpty { allCandidates }
        pool.distinctBy { "${it.pluginId}:${it.summary.nativeId}" }.take(6)
    }

    val visibleContinueWatching = remember(continueWatching, livePluginIds) {
        continueWatching.filterNot {
            it.mediaType == "live" || it.progress.media.pluginId in livePluginIds
        }
    }
    val hasEnabledPlugins = homeEligibleIds.isNotEmpty()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase),
        contentPadding = PaddingValues(bottom = 24.dp + LocalVesselFloatingNavInset.current),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        // Native FFI warning banner if .so isn't built yet
        initError?.let { err ->
            item {
                Surface(
                    color = palette.danger.copy(alpha = 0.14f),
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(VesselI18n.tr("home.engine_setup_required"), fontWeight = FontWeight.Bold, color = palette.danger)
                        Text(err, style = MaterialTheme.typography.bodySmall, color = palette.textPrimary)
                    }
                }
            }
        }

        // CloudStream-style Full-Bleed Hero Stage with Ken Burns zoom & smooth looping swiper
        if (hasEnabledPlugins && prefs.home.heroBanner != "hidden") {
            val heroHeight = if (prefs.home.heroBanner == "compact") 280.dp else 400.dp
            item(key = "spotlight-hero") {
                val isTv = LocalIsTvDevice.current
                val feedsBtnFocus = remember { FocusRequester() }
                val refreshBtnFocus = remember { FocusRequester() }
                val heroWatchFocus = remember { FocusRequester() }
                val heroInfoFocus = remember { FocusRequester() }
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(heroHeight)
                        .animateContentSize(tween(280, easing = FastOutSlowInEasing))
                        .background(palette.bgSurface),
                ) {
                    if (spotlightPool.isNotEmpty()) {
                        val poolCount = spotlightPool.size
                        val loopEnabled = poolCount > 1
                        val virtualCount = if (loopEnabled) poolCount * 1000 else 1
                        val initialVirtualPage = if (loopEnabled) poolCount * 500 else 0
                        val pagerState = rememberPagerState(
                            initialPage = initialVirtualPage,
                            pageCount = { virtualCount },
                        )

                        // Key ONLY on settledPage & poolCount so starting animateScrollToPage does NOT cancel itself via isScrollInProgress.
                        LaunchedEffect(poolCount, pagerState.settledPage) {
                            if (!loopEnabled) return@LaunchedEffect
                            delay(5300)
                            while (pagerState.isScrollInProgress) {
                                delay(150)
                            }
                            val next = (pagerState.settledPage + 1).coerceAtMost(virtualCount - 1)
                            pagerState.animateScrollToPage(
                                page = next,
                                animationSpec = tween(durationMillis = 820, easing = FastOutSlowInEasing),
                            )
                        }

                        HorizontalPager(
                            state = pagerState,
                            beyondViewportPageCount = 1,
                            modifier = Modifier.fillMaxSize(),
                        ) { page ->
                            val logicalIdx = ((page % poolCount) + poolCount) % poolCount
                            val hero = spotlightPool[logicalIdx]
                            val detailKey = "${hero.pluginId}:${hero.summary.nativeId}"
                            val detail = heroDetails[detailKey]

                            LaunchedEffect(detailKey) {
                                if (!heroDetails.containsKey(detailKey)) {
                                    runCatching { repo.getDetails(hero.pluginId, hero.summary.nativeId) }
                                        .onSuccess { heroDetails[detailKey] = it }
                                }
                            }

                            val fallbackUrl = detail?.poster?.takeIf { it.isNotBlank() } ?: hero.summary.poster
                            val backdropUrl = detail?.backdrop?.takeIf { it.isNotBlank() && it != fallbackUrl }

                            // Slow cinematic Ken Burns zoom on the active slide
                            val zoomAnim = remember(page) { Animatable(1.01f) }
                            val isActiveSlide = pagerState.settledPage == page
                            LaunchedEffect(isActiveSlide, page, poolCount) {
                                if (isActiveSlide) {
                                    zoomAnim.snapTo(1.01f)
                                    zoomAnim.animateTo(
                                        targetValue = 1.08f,
                                        animationSpec = tween(durationMillis = 5200, easing = LinearEasing),
                                    )
                                } else {
                                    zoomAnim.animateTo(1.01f, tween(350))
                                }
                            }

                            val pageOffset = ((pagerState.currentPage - page) + pagerState.currentPageOffsetFraction)
                            val slideAlpha = (1f - abs(pageOffset) * 0.42f).coerceIn(0.45f, 1f)

                            Box(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .clip(RoundedCornerShape(0.dp))
                                    .then(
                                        if (isTv) Modifier
                                        else Modifier.clickable { onOpenDetails(hero.pluginId, hero.summary.nativeId) },
                                    ),
                            ) {
                                if (!fallbackUrl.isNullOrBlank()) {
                                    AsyncImage(
                                        model = fallbackUrl,
                                        contentDescription = hero.summary.title,
                                        contentScale = ContentScale.Crop,
                                        modifier = Modifier
                                            .fillMaxSize()
                                            .graphicsLayer {
                                                scaleX = zoomAnim.value
                                                scaleY = zoomAnim.value
                                                translationX = pageOffset * size.width * 0.14f
                                                alpha = slideAlpha
                                            },
                                    )
                                }
                                AnimatedVisibility(
                                    visible = !backdropUrl.isNullOrBlank(),
                                    enter = fadeIn(tween(500, easing = FastOutSlowInEasing)),
                                    exit = fadeOut(tween(300)),
                                    modifier = Modifier.fillMaxSize(),
                                ) {
                                    if (!backdropUrl.isNullOrBlank()) {
                                        AsyncImage(
                                            model = backdropUrl,
                                            contentDescription = hero.summary.title,
                                            contentScale = ContentScale.Crop,
                                            modifier = Modifier
                                                .fillMaxSize()
                                                .graphicsLayer {
                                                    scaleX = zoomAnim.value
                                                    scaleY = zoomAnim.value
                                                    translationX = pageOffset * size.width * 0.14f
                                                    alpha = slideAlpha
                                                },
                                        )
                                    }
                                }
                                if (fallbackUrl.isNullOrBlank() && backdropUrl.isNullOrBlank()) {
                                    Box(
                                        modifier = Modifier
                                            .fillMaxSize()
                                            .background(
                                                Brush.linearGradient(
                                                    listOf(
                                                        palette.accent.copy(alpha = 0.28f),
                                                        palette.bgElevated,
                                                        palette.bgBase,
                                                    ),
                                                ),
                                            ),
                                    )
                                }

                                // Subtle top scrim for floating status / action controls
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(88.dp)
                                        .align(Alignment.TopCenter)
                                        .background(
                                            Brush.verticalGradient(
                                                listOf(Color.Black.copy(alpha = 0.56f), Color.Transparent),
                                            ),
                                        ),
                                )

                                // Seamless CloudStream-style bottom gradient dissolving into bgBase
                                Box(
                                    modifier = Modifier
                                        .fillMaxSize()
                                        .background(
                                            Brush.verticalGradient(
                                                0.0f to Color.Transparent,
                                                0.38f to Color.Black.copy(alpha = 0.22f),
                                                0.74f to palette.bgBase.copy(alpha = 0.84f),
                                                1.0f to palette.bgBase,
                                            ),
                                        ),
                                )

                                Column(
                                    modifier = Modifier
                                        .align(Alignment.BottomStart)
                                        .fillMaxWidth()
                                        .padding(start = 16.dp, end = 16.dp, bottom = if (loopEnabled) 28.dp else 14.dp),
                                    verticalArrangement = Arrangement.spacedBy(7.dp),
                                ) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(7.dp),
                                    ) {
                                        Surface(
                                            color = palette.accent.copy(alpha = 0.9f),
                                            shape = RoundedCornerShape(6.dp),
                                        ) {
                                            Text(
                                                text = if (hero.summary.isAnime) "ANIME" else hero.summary.type.uppercase(Locale.US),
                                                style = MaterialTheme.typography.labelSmall,
                                                fontWeight = FontWeight.Bold,
                                                color = Color(0xFF11111B),
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                            )
                                        }
                                        val year = detail?.year ?: hero.summary.year
                                        if (year != null) {
                                            Text(
                                                text = year.toString(),
                                                style = MaterialTheme.typography.labelMedium,
                                                color = Color.White.copy(alpha = 0.9f),
                                            )
                                        }
                                        val rating = detail?.rating ?: hero.summary.rating
                                        if (rating != null) {
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(2.dp),
                                            ) {
                                                Icon(
                                                    Icons.Default.Star,
                                                    contentDescription = null,
                                                    tint = palette.warning,
                                                    modifier = Modifier.size(13.dp),
                                                )
                                                Text(
                                                    text = String.format(Locale.US, "%.1f", rating),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    fontWeight = FontWeight.SemiBold,
                                                    color = Color.White,
                                                )
                                            }
                                        }
                                        Surface(
                                            color = Color.Black.copy(alpha = 0.45f),
                                            shape = RoundedCornerShape(6.dp),
                                        ) {
                                            Text(
                                                text = hero.pluginName,
                                                style = MaterialTheme.typography.labelSmall,
                                                color = Color.White.copy(alpha = 0.88f),
                                                maxLines = 1,
                                                overflow = TextOverflow.Ellipsis,
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                            )
                                        }
                                    }

                                    Text(
                                        text = hero.summary.title,
                                        style = MaterialTheme.typography.headlineMedium,
                                        fontWeight = FontWeight.ExtraBold,
                                        color = Color.White,
                                        maxLines = 2,
                                        overflow = TextOverflow.Ellipsis,
                                    )

                                    val subLine = detail?.genres?.take(3)?.takeIf { it.isNotEmpty() }?.joinToString(" • ")
                                        ?: detail?.synopsis?.takeIf { it.isNotBlank() }
                                    if (!subLine.isNullOrBlank()) {
                                        Text(
                                            text = subLine,
                                            style = MaterialTheme.typography.bodySmall,
                                            color = Color.White.copy(alpha = 0.78f),
                                            maxLines = 2,
                                            overflow = TextOverflow.Ellipsis,
                                        )
                                    }

                                    val isHeroLaunching = launchingHeroKey == detailKey
                                    val homeTokens = LocalVesselUiTokens.current
                                    val isCurrentHeroPage = pagerState.currentPage == page
                                    Row(
                                        modifier = Modifier.padding(top = 4.dp),
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    ) {
                                        VesselButton(
                                            onClick = { watchHeroNow(hero.pluginId, hero.summary.nativeId) },
                                            enabled = launchingHeroKey == null,
                                            height = homeTokens.compactButtonHeight,
                                            contentPadding = PaddingValues(horizontal = 13.dp, vertical = 0.dp),
                                            modifier = Modifier
                                                .then(if (isCurrentHeroPage) Modifier.focusRequester(heroWatchFocus) else Modifier)
                                                .focusProperties {
                                                    up = feedsBtnFocus
                                                    if (isCurrentHeroPage) {
                                                        right = heroInfoFocus
                                                    }
                                                },
                                        ) {
                                            if (isHeroLaunching) {
                                                CircularProgressIndicator(
                                                    modifier = Modifier.size(14.dp),
                                                    strokeWidth = 2.dp,
                                                    color = Color(0xFF11111B),
                                                )
                                            } else {
                                                Icon(Icons.Default.PlayArrow, contentDescription = null, modifier = Modifier.size(16.dp))
                                            }
                                            Spacer(Modifier.width(5.dp))
                                            Text(
                                                text = VesselI18n.tr("home.watch_now"),
                                                style = MaterialTheme.typography.labelMedium,
                                                fontWeight = FontWeight.Bold,
                                            )
                                        }
                                        Surface(
                                            shape = RoundedCornerShape(homeTokens.cornerRadius),
                                            color = Color.Black.copy(alpha = 0.44f),
                                            border = BorderStroke(1.dp, Color.White.copy(alpha = 0.30f)),
                                            modifier = Modifier
                                                .height(homeTokens.compactButtonHeight)
                                                .clip(RoundedCornerShape(homeTokens.cornerRadius))
                                                .then(if (isCurrentHeroPage) Modifier.focusRequester(heroInfoFocus) else Modifier)
                                                .focusProperties {
                                                    up = feedsBtnFocus
                                                    if (isCurrentHeroPage) {
                                                        left = heroWatchFocus
                                                    }
                                                }
                                                .vesselFocusBorder(RoundedCornerShape(homeTokens.cornerRadius))
                                                .clickable { onOpenDetails(hero.pluginId, hero.summary.nativeId) },
                                        ) {
                                            Row(
                                                verticalAlignment = Alignment.CenterVertically,
                                                modifier = Modifier.padding(horizontal = 12.dp),
                                            ) {
                                                Icon(Icons.Default.Info, contentDescription = null, tint = Color.White, modifier = Modifier.size(15.dp))
                                                Spacer(Modifier.width(5.dp))
                                                Text(
                                                    text = VesselI18n.tr("home.more_info"),
                                                    style = MaterialTheme.typography.labelMedium,
                                                    color = Color.White,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if (loopEnabled) {
                            val activeLogical = ((pagerState.currentPage % poolCount) + poolCount) % poolCount
                            Row(
                                modifier = Modifier
                                    .align(Alignment.BottomCenter)
                                    .padding(bottom = 10.dp),
                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                repeat(poolCount) { index ->
                                    val active = activeLogical == index
                                    val barWidth by animateDpAsState(
                                        targetValue = if (active) 22.dp else 6.dp,
                                        animationSpec = tween(280, easing = FastOutSlowInEasing),
                                        label = "hero_ind_w",
                                    )
                                    val barColor by animateColorAsState(
                                        targetValue = if (active) palette.accent else Color.White.copy(alpha = 0.38f),
                                        animationSpec = tween(240),
                                        label = "hero_ind_c",
                                    )
                                    Box(
                                        modifier = Modifier
                                            .height(4.dp)
                                            .width(barWidth)
                                            .clip(CircleShape)
                                            .background(barColor)
                                            .then(
                                                if (isTv) Modifier
                                                else Modifier.clickable {
                                                    val diff = index - activeLogical
                                                    scope.launch {
                                                        pagerState.animateScrollToPage(
                                                            pagerState.currentPage + diff,
                                                            animationSpec = tween(650, easing = FastOutSlowInEasing),
                                                        )
                                                    }
                                                },
                                            ),
                                    )
                                }
                            }
                        }
                    } else if (loading) {
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .background(
                                    Brush.verticalGradient(
                                        listOf(palette.bgElevated, palette.bgBase),
                                    ),
                                ),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator(color = palette.accent, modifier = Modifier.size(28.dp), strokeWidth = 2.5.dp)
                        }
                    } else {
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .background(
                                    Brush.verticalGradient(
                                        listOf(palette.bgElevated, palette.bgBase),
                                    ),
                                ),
                            contentAlignment = Alignment.Center,
                        ) {
                            Column(
                                modifier = Modifier.padding(horizontal = 24.dp),
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                Text(
                                    text = VesselI18n.tr("home.empty_selected"),
                                    color = palette.textMuted,
                                    textAlign = TextAlign.Center,
                                )
                                VesselButton(onClick = { showFeedsModal = true }) {
                                    Text(VesselI18n.tr("home.choose_feeds"))
                                }
                            }
                        }
                    }

                    // Floating compact Source & Refresh controls over top-right of Hero with safe top cutout inset
                    val heroTopTokens = LocalVesselUiTokens.current
                    Row(
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .windowInsetsPadding(WindowInsets.statusBarsIgnoringVisibility.union(WindowInsets.displayCutout))
                            .padding(horizontal = 12.dp, vertical = 10.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Surface(
                            shape = RoundedCornerShape(999.dp),
                            color = Color.Black.copy(alpha = 0.48f),
                            border = BorderStroke(1.dp, Color.White.copy(alpha = 0.18f)),
                            modifier = Modifier
                                .height(heroTopTokens.smallIconButtonSize)
                                .clip(RoundedCornerShape(999.dp))
                                .focusRequester(feedsBtnFocus)
                                .focusProperties {
                                    right = refreshBtnFocus
                                    if (spotlightPool.isNotEmpty()) {
                                        down = heroWatchFocus
                                    }
                                }
                                .vesselFocusBorder(RoundedCornerShape(999.dp))
                                .clickable { showFeedsModal = true },
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 11.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                            ) {
                                Icon(Icons.Default.Tune, contentDescription = VesselI18n.tr("home.choose_feeds"), tint = Color.White, modifier = Modifier.size(15.dp))
                                Text(
                                    text = "${VesselI18n.tr("home.feeds")} (${effectiveHomeIds.size})",
                                    color = Color.White,
                                    style = MaterialTheme.typography.labelMedium,
                                    fontWeight = FontWeight.SemiBold,
                                )
                            }
                        }
                        VesselCompactIconButton(
                            onClick = { scope.launch { loadHome(forceRefresh = true) } },
                            size = heroTopTokens.smallIconButtonSize,
                            containerColor = Color.Black.copy(alpha = 0.48f),
                            borderColor = Color.White.copy(alpha = 0.18f),
                            modifier = Modifier
                                .focusRequester(refreshBtnFocus)
                                .focusProperties {
                                    left = feedsBtnFocus
                                    if (spotlightPool.isNotEmpty()) {
                                        down = heroWatchFocus
                                    }
                                },
                        ) {
                            Icon(
                                Icons.Default.Refresh,
                                contentDescription = VesselI18n.tr("common.refresh"),
                                tint = Color.White,
                                modifier = Modifier.size(16.dp),
                            )
                        }
                    }
                }
            }
        } else if (hasEnabledPlugins) {
            item {
                val topTokens = LocalVesselUiTokens.current
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .windowInsetsPadding(WindowInsets.statusBarsIgnoringVisibility.union(WindowInsets.displayCutout))
                        .padding(start = 16.dp, top = 12.dp, end = 16.dp, bottom = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    VesselOutlinedButton(
                        onClick = { showFeedsModal = true },
                        height = topTokens.smallIconButtonSize,
                        contentPadding = PaddingValues(horizontal = 11.dp, vertical = 0.dp),
                    ) {
                        Icon(Icons.Default.Tune, contentDescription = null, modifier = Modifier.size(15.dp))
                        Spacer(Modifier.width(5.dp))
                        Text("${VesselI18n.tr("home.feeds")} (${effectiveHomeIds.size})", style = MaterialTheme.typography.labelMedium)
                    }
                    VesselCompactIconButton(
                        onClick = { scope.launch { loadHome(forceRefresh = true) } },
                        size = topTokens.smallIconButtonSize,
                        containerColor = palette.bgSurface,
                        borderColor = palette.border.copy(alpha = 0.65f),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = VesselI18n.tr("common.refresh"), modifier = Modifier.size(16.dp))
                    }
                }
            }
        }

        // Continue Watching Rail
        if (prefs.home.showContinueWatching && visibleContinueWatching.isNotEmpty()) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(
                        text = VesselI18n.tr("home.continue_watching"),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(horizontal = 16.dp),
                    )
                    LazyRow(
                        contentPadding = PaddingValues(horizontal = 16.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        items(visibleContinueWatching, key = { "${it.progress.media.pluginId}:${it.progress.media.nativeId}" }) { entry ->
                            ContinueWatchingCard(
                                entry = entry,
                                onResume = { onResumeEntry(entry) },
                                onGoToDetails = {
                                    onOpenDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                },
                                onEdit = {
                                    editingCwEntry = entry
                                },
                                onRemove = {
                                    scope.launch {
                                        repo.deleteWatchProgress(
                                            entry.progress.media.pluginId,
                                            entry.progress.media.nativeId,
                                        )
                                        loadHome()
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }

        if (!hasEnabledPlugins) {
            item {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 24.dp, vertical = 72.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(14.dp),
                ) {
                    VesselLogoMark(modifier = Modifier.size(56.dp))
                    Text(
                        text = VesselI18n.tr("home.hero_title"),
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                        textAlign = TextAlign.Center,
                    )
                    Text(
                        text = VesselI18n.tr("home.empty_plugins"),
                        color = palette.textMuted,
                        textAlign = TextAlign.Center,
                    )
                    VesselButton(onClick = onOpenExtensions) {
                        Text(VesselI18n.tr("nav.extensions"))
                    }
                }
            }
        } else if (loading && orderedShelves.isEmpty()) {
            item {
                Box(modifier = Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(modifier = Modifier.size(26.dp), strokeWidth = 2.5.dp)
                }
            }
        } else {
            orderedShelves.forEach { result ->
                if (!result.error.isNullOrBlank()) {
                    item(key = "${result.pluginId}:err") {
                        Surface(
                            color = palette.danger.copy(alpha = 0.12f),
                            shape = RoundedCornerShape(12.dp),
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                        ) {
                            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Text(
                                    text = "${result.pluginName} • ${VesselI18n.tr("home.plugin_error", "Feed error")}",
                                    style = MaterialTheme.typography.labelLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = palette.danger,
                                )
                                Text(result.error, style = MaterialTheme.typography.bodySmall, color = palette.textPrimary)
                            }
                        }
                    }
                }
                result.sections.forEach { section ->
                    if (section.items.isNotEmpty()) {
                        item(key = "${result.pluginId}:${section.id}") {
                            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = section.title,
                                        style = MaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                    )
                                    Text(
                                        text = result.pluginName,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                    )
                                }
                                LazyRow(
                                    contentPadding = PaddingValues(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                                ) {
                                    items(section.items, key = { "${result.pluginId}:${it.nativeId}" }) { item ->
                                        val pid = item.pluginId ?: result.pluginId
                                        MediaPosterCard(
                                            item = item,
                                            cards = prefs.cards,
                                            onClick = { onOpenDetails(pid, item.nativeId) },
                                            onQuickStatus = { status ->
                                                scope.launch {
                                                    repo.upsertLibrary(
                                                        LibraryItem(
                                                            media = MediaRef(pid, item.nativeId),
                                                            mediaType = item.type,
                                                            isAnime = item.isAnime,
                                                            title = item.title,
                                                            posterUrl = item.poster,
                                                            status = status,
                                                        ),
                                                    )
                                                    repo.notify("${item.title} · ${VesselI18n.tr("library.$status")}")
                                                }
                                            },
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showFeedsModal) {
        val grouped = remember(homeEligiblePlugins) {
            homeEligiblePlugins.groupBy(::resolveLiveGroupName).toList()
        }
        var draftSelection by remember(showFeedsModal) { mutableStateOf(effectiveHomeIds) }
        var draftHeroPlugins by remember(showFeedsModal) {
            mutableStateOf(effectiveHeroIds)
        }
        var draftHeroBanner by remember(showFeedsModal) {
            mutableStateOf(prefs.home.heroBanner)
        }
        var draggingIndex by remember { mutableIntStateOf(-1) }
        var dragAccumY by remember { mutableFloatStateOf(0f) }
        var rowHeightPx by remember { mutableIntStateOf(120) }

        fun moveDraftItem(from: Int, to: Int) {
            if (from == to || from !in draftSelection.indices || to !in draftSelection.indices) return
            val next = draftSelection.toMutableList()
            val item = next.removeAt(from)
            next.add(to, item)
            draftSelection = next
        }

        fun toggleDraftHome(pid: String) {
            draftSelection = if (pid in draftSelection) {
                draftSelection.filterNot { it == pid }
            } else {
                draftSelection + pid
            }
        }

        fun toggleDraftHero(pid: String) {
            draftHeroPlugins = if (pid in draftHeroPlugins) {
                draftHeroPlugins.filterNot { it == pid }
            } else {
                draftHeroPlugins + pid
            }
        }

        fun toggleGroup(items: List<dev.falsis.vessel.data.PluginListEntry>, enable: Boolean) {
            val ids = items.map { it.manifest.id }
            draftSelection = if (enable) {
                val cur = draftSelection.toMutableList()
                for (id in ids) {
                    if (id !in cur) cur.add(id)
                }
                cur
            } else {
                val removeSet = ids.toSet()
                draftSelection.filterNot { it in removeSet }
            }
        }

        Dialog(
            onDismissRequest = { showFeedsModal = false },
            properties = DialogProperties(usePlatformDefaultWidth = false),
        ) {
            Surface(
                shape = RoundedCornerShape(18.dp),
                color = palette.bgSurface,
                border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.30f)),
                shadowElevation = 24.dp,
                modifier = Modifier
                    .widthIn(max = 680.dp)
                    .fillMaxWidth(0.94f)
                    .fillMaxHeight(0.86f),
            ) {
                Column(modifier = Modifier.fillMaxSize()) {
                    // Modal Header (Desktop .picker-head)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 18.dp, vertical = 14.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.Top,
                    ) {
                        Column(
                            modifier = Modifier.weight(1f).padding(end = 8.dp),
                            verticalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Text(
                                text = VesselI18n.tr("home.modal_title"),
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = palette.textPrimary,
                            )
                            Text(
                                text = VesselI18n.tr("home.modal_subtitle"),
                                style = MaterialTheme.typography.bodySmall,
                                color = palette.textMuted,
                            )
                        }
                        Surface(
                            shape = CircleShape,
                            color = palette.bgElevated,
                            modifier = Modifier
                                .size(34.dp)
                                .clip(CircleShape)
                                .clickable { showFeedsModal = false },
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                Icon(
                                    Icons.Default.Close,
                                    contentDescription = VesselI18n.tr("common.close"),
                                    tint = palette.textPrimary,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                    }

                    HorizontalDivider(color = palette.border.copy(alpha = 0.45f))

                    // Scrollable Body (Desktop .picker-groups)
                    Column(
                        modifier = Modifier
                            .weight(1f)
                            .verticalScroll(rememberScrollState())
                            .padding(horizontal = 16.dp, vertical = 14.dp),
                        verticalArrangement = Arrangement.spacedBy(14.dp),
                    ) {
                        // 1. Spotlight Hero Source Section (Desktop .hero-source-section)
                        Surface(
                            shape = RoundedCornerShape(12.dp),
                            color = palette.accent.copy(alpha = 0.06f),
                            border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.22f)),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Column(
                                modifier = Modifier.padding(12.dp),
                                verticalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                Text(
                                    text = VesselI18n.tr("settings.hero_banner"),
                                    fontWeight = FontWeight.Bold,
                                    style = MaterialTheme.typography.labelLarge,
                                    color = palette.textPrimary,
                                )
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    verticalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    listOf("full", "compact", "hidden").forEach { mode ->
                                        VesselFilterChip(
                                            selected = draftHeroBanner == mode,
                                            onClick = { draftHeroBanner = mode },
                                            label = VesselI18n.tr("settings.hero_$mode"),
                                        )
                                    }
                                }
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = VesselI18n.tr("home.hero_source_title", "Öne Çıkan Vitrin Kaynağı"),
                                        fontWeight = FontWeight.Bold,
                                        style = MaterialTheme.typography.labelLarge,
                                        color = palette.textPrimary,
                                        modifier = Modifier.weight(1f).padding(end = 8.dp),
                                    )
                                    VesselOutlinedButton(
                                        enabled = draftHeroPlugins.isNotEmpty(),
                                        onClick = { draftHeroPlugins = emptyList() },
                                        height = 28.dp,
                                        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp),
                                    ) {
                                        Text(
                                            text = VesselI18n.tr("home.hero_source_auto"),
                                            style = MaterialTheme.typography.labelSmall,
                                        )
                                    }
                                }
                                Text(
                                    text = VesselI18n.tr("home.hero_source_hint"),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = palette.textMuted,
                                )
                                FlowRow(
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    verticalArrangement = Arrangement.spacedBy(6.dp),
                                ) {
                                    homeEligiblePlugins.forEach { entry ->
                                        val pid = entry.manifest.id
                                        val selected = pid in draftHeroPlugins
                                        Surface(
                                            shape = RoundedCornerShape(999.dp),
                                            color = if (selected) palette.accent.copy(alpha = 0.20f) else palette.bgElevated,
                                            border = BorderStroke(
                                                width = 1.dp,
                                                color = if (selected) palette.accent else palette.border.copy(alpha = 0.7f),
                                            ),
                                            modifier = Modifier
                                                .clip(RoundedCornerShape(999.dp))
                                                .clickable { toggleDraftHero(pid) },
                                        ) {
                                            Row(
                                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                                verticalAlignment = Alignment.CenterVertically,
                                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                                            ) {
                                                Icon(
                                                    Icons.Default.AutoAwesome,
                                                    contentDescription = null,
                                                    tint = if (selected) palette.accent else palette.textMuted,
                                                    modifier = Modifier.size(13.dp),
                                                )
                                                Text(
                                                    text = entry.manifest.name,
                                                    style = MaterialTheme.typography.labelSmall,
                                                    fontWeight = FontWeight.SemiBold,
                                                    color = if (selected) palette.textPrimary else palette.textMuted,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 2. Active Home Shelf Order Section (Desktop .order-section)
                        if (draftSelection.isNotEmpty()) {
                            Surface(
                                shape = RoundedCornerShape(12.dp),
                                color = palette.accent.copy(alpha = 0.06f),
                                border = BorderStroke(1.dp, palette.accent.copy(alpha = 0.22f)),
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                Column(
                                    modifier = Modifier.padding(12.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                        Text(
                                            text = VesselI18n.tr("home.shelf_order_title"),
                                            fontWeight = FontWeight.Bold,
                                            style = MaterialTheme.typography.labelLarge,
                                            color = palette.textPrimary,
                                        )
                                        Text(
                                            text = VesselI18n.tr("home.shelf_order_hint"),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = palette.textMuted,
                                        )
                                    }

                                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                        draftSelection.forEachIndexed { idx, pid ->
                                            val isDragging = draggingIndex == idx
                                            Surface(
                                                shape = RoundedCornerShape(9.dp),
                                                color = if (isDragging) palette.accent.copy(alpha = 0.16f) else palette.bgElevated,
                                                border = BorderStroke(
                                                    width = 1.dp,
                                                    color = if (isDragging) palette.accent else palette.border.copy(alpha = 0.7f),
                                                ),
                                                modifier = Modifier
                                                    .fillMaxWidth()
                                                    .onSizeChanged { sz ->
                                                        if (sz.height > 0) rowHeightPx = sz.height
                                                    },
                                            ) {
                                                Row(
                                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                                                    verticalAlignment = Alignment.CenterVertically,
                                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                                ) {
                                                    Icon(
                                                        Icons.Default.DragIndicator,
                                                        contentDescription = VesselI18n.tr("home.shelf_order_hint"),
                                                        tint = palette.textMuted,
                                                        modifier = Modifier
                                                            .size(22.dp)
                                                            .pointerInput(pid) {
                                                                detectVerticalDragGestures(
                                                                    onDragStart = {
                                                                        draggingIndex = draftSelection.indexOf(pid)
                                                                        dragAccumY = 0f
                                                                    },
                                                                    onDragEnd = {
                                                                        draggingIndex = -1
                                                                        dragAccumY = 0f
                                                                    },
                                                                    onDragCancel = {
                                                                        draggingIndex = -1
                                                                        dragAccumY = 0f
                                                                    },
                                                                    onVerticalDrag = { _, dragAmount ->
                                                                        val cur = draftSelection.indexOf(pid)
                                                                        if (cur == -1) return@detectVerticalDragGestures
                                                                        dragAccumY += dragAmount
                                                                        val stepPx = (rowHeightPx + 12).coerceAtLeast(60)
                                                                        if (dragAccumY > stepPx * 0.55f && cur < draftSelection.lastIndex) {
                                                                            moveDraftItem(cur, cur + 1)
                                                                            draggingIndex = cur + 1
                                                                            dragAccumY -= stepPx
                                                                        } else if (dragAccumY < -stepPx * 0.55f && cur > 0) {
                                                                            moveDraftItem(cur, cur - 1)
                                                                            draggingIndex = cur - 1
                                                                            dragAccumY += stepPx
                                                                        }
                                                                    },
                                                                )
                                                            },
                                                    )
                                                    Box(
                                                        modifier = Modifier
                                                            .size(22.dp)
                                                            .clip(RoundedCornerShape(6.dp))
                                                            .background(palette.accent.copy(alpha = 0.18f)),
                                                        contentAlignment = Alignment.Center,
                                                    ) {
                                                        Text(
                                                            text = "${idx + 1}",
                                                            color = palette.accent,
                                                            style = MaterialTheme.typography.labelSmall,
                                                            fontWeight = FontWeight.Bold,
                                                        )
                                                    }
                                                    Text(
                                                        text = pluginNameById[pid] ?: pid,
                                                        color = palette.textPrimary,
                                                        style = MaterialTheme.typography.bodySmall,
                                                        fontWeight = FontWeight.SemiBold,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                        modifier = Modifier.weight(1f),
                                                    )
                                                    IconButton(
                                                        enabled = idx > 0,
                                                        onClick = { moveDraftItem(idx, idx - 1) },
                                                        modifier = Modifier.size(26.dp),
                                                    ) {
                                                        Icon(
                                                            Icons.Default.KeyboardArrowUp,
                                                            contentDescription = "Move Up",
                                                            tint = if (idx > 0) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                            modifier = Modifier.size(18.dp),
                                                        )
                                                    }
                                                    IconButton(
                                                        enabled = idx < draftSelection.lastIndex,
                                                        onClick = { moveDraftItem(idx, idx + 1) },
                                                        modifier = Modifier.size(26.dp),
                                                    ) {
                                                        Icon(
                                                            Icons.Default.KeyboardArrowDown,
                                                            contentDescription = "Move Down",
                                                            tint = if (idx < draftSelection.lastIndex) palette.textPrimary else palette.textMuted.copy(alpha = 0.35f),
                                                            modifier = Modifier.size(18.dp),
                                                        )
                                                    }
                                                    IconButton(
                                                        onClick = { toggleDraftHome(pid) },
                                                        modifier = Modifier.size(26.dp),
                                                    ) {
                                                        Icon(
                                                            Icons.Default.Close,
                                                            contentDescription = VesselI18n.tr("common.remove"),
                                                            tint = palette.textMuted,
                                                            modifier = Modifier.size(15.dp),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 3. Grouped Enabled Plugins (Desktop .picker-group + .picker-grid)
                        grouped.forEach { (groupName, groupItems) ->
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = groupName,
                                        fontWeight = FontWeight.Bold,
                                        style = MaterialTheme.typography.labelLarge,
                                        color = palette.textPrimary,
                                    )
                                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                        VesselOutlinedButton(
                                            onClick = { toggleGroup(groupItems, true) },
                                            height = 26.dp,
                                            contentPadding = PaddingValues(horizontal = 9.dp, vertical = 0.dp),
                                        ) {
                                            Text(VesselI18n.tr("common.all"), style = MaterialTheme.typography.labelSmall)
                                        }
                                        VesselOutlinedButton(
                                            onClick = { toggleGroup(groupItems, false) },
                                            height = 26.dp,
                                            contentPadding = PaddingValues(horizontal = 9.dp, vertical = 0.dp),
                                        ) {
                                            Text(VesselI18n.tr("common.none"), style = MaterialTheme.typography.labelSmall)
                                        }
                                    }
                                }

                                groupItems.chunked(2).forEach { rowItems ->
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    ) {
                                        rowItems.forEach { entry ->
                                            val pid = entry.manifest.id
                                            val checked = pid in draftSelection
                                            Surface(
                                                shape = RoundedCornerShape(10.dp),
                                                color = if (checked) palette.accent.copy(alpha = 0.14f) else palette.bgElevated,
                                                border = BorderStroke(
                                                    width = 1.dp,
                                                    color = if (checked) palette.accent else palette.border.copy(alpha = 0.7f),
                                                ),
                                                modifier = Modifier
                                                    .weight(1f)
                                                    .clip(RoundedCornerShape(10.dp))
                                                    .clickable { toggleDraftHome(pid) },
                                            ) {
                                                Row(
                                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                                                    verticalAlignment = Alignment.CenterVertically,
                                                ) {
                                                    Checkbox(
                                                        checked = checked,
                                                        onCheckedChange = { toggleDraftHome(pid) },
                                                        modifier = Modifier.size(32.dp),
                                                    )
                                                    Spacer(Modifier.width(4.dp))
                                                    Text(
                                                        text = entry.manifest.name,
                                                        style = MaterialTheme.typography.bodySmall,
                                                        fontWeight = FontWeight.Medium,
                                                        color = palette.textPrimary,
                                                        maxLines = 1,
                                                        overflow = TextOverflow.Ellipsis,
                                                        modifier = Modifier.weight(1f),
                                                    )
                                                }
                                            }
                                        }
                                        if (rowItems.size == 1) {
                                            Spacer(Modifier.weight(1f))
                                        }
                                    }
                                }
                            }
                        }
                    }

                    HorizontalDivider(color = palette.border.copy(alpha = 0.45f))

                    // Modal Footer (Desktop .picker-foot)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "${draftSelection.size} ${VesselI18n.tr("home.active_count", "aktif")}",
                            style = MaterialTheme.typography.labelMedium,
                            color = palette.textMuted,
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            VesselTextButton(onClick = { showFeedsModal = false }) {
                                Text(VesselI18n.tr("common.cancel"))
                            }
                            VesselButton(
                                height = 34.dp,
                                onClick = {
                                    showFeedsModal = false
                                    val nextSelected = draftSelection.toList()
                                    val nextHero = draftHeroPlugins.toList().takeIf { it.isNotEmpty() }
                                    scope.launch {
                                        repo.saveUiPreferences(
                                            prefs.copy(
                                                home = prefs.home.copy(
                                                    heroBanner = draftHeroBanner,
                                                    selectedPlugins = nextSelected,
                                                    heroPlugins = nextHero,
                                                ),
                                            ),
                                        )
                                        loadHome(forceRefresh = true)
                                    }
                                },
                            ) {
                                Text(VesselI18n.tr("common.save"))
                            }
                        }
                    }
                }
            }
        }
    }

    heroSourceModal?.let { modal ->
        AlertDialog(
            onDismissRequest = { heroSourceModal = null },
            title = {
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(VesselI18n.tr("details.select_stream"), fontWeight = FontWeight.Bold)
                    Text(
                        text = if (modal.detail.type == "movie") {
                            modal.detail.title
                        } else {
                            "${modal.detail.title} · S${modal.episode.season}:E${modal.episode.episode}"
                        },
                        style = MaterialTheme.typography.labelMedium,
                        color = palette.textMuted,
                    )
                }
            },
            text = {
                Column(
                    modifier = Modifier.fillMaxWidth().heightIn(max = 360.dp).verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    modal.streamResult.streams.forEachIndexed { idx, cand ->
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = palette.bgElevated,
                            modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).clickable {
                                heroSourceModal = null
                                onPlayHero(modal.pluginId, modal.nativeId, modal.detail, modal.episode, modal.streamResult, idx, modal.resumePosMs)
                            },
                        ) {
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Text(cand.label, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
                                    Text(
                                        listOfNotNull(cand.pluginName ?: cand.pluginId, cand.quality, cand.kind.uppercase(Locale.US), cand.audioType)
                                            .joinToString(" · "),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = palette.textMuted,
                                    )
                                }
                                Icon(Icons.Default.PlayArrow, contentDescription = null, tint = palette.accent, modifier = Modifier.size(18.dp))
                            }
                        }
                    }
                }
            },
            confirmButton = {
                VesselTextButton(onClick = { heroSourceModal = null }) {
                    Text(VesselI18n.tr("common.close"))
                }
            },
        )
    }

    editingCwEntry?.let { entry ->
        var episodesList by remember(entry) { mutableStateOf<List<Episode>>(emptyList()) }
        var progressRows by remember(entry) { mutableStateOf<List<PlaybackProgress>>(emptyList()) }
        var loadingEps by remember(entry) { mutableStateOf(entry.mediaType != "movie") }
        var savingEdit by remember(entry) { mutableStateOf(false) }
        var showEpMenu by remember(entry) { mutableStateOf(false) }
        var seasonText by remember(entry) { mutableStateOf((if (entry.season > 0) entry.season else 1).toString()) }
        var episodeText by remember(entry) { mutableStateOf((if (entry.episode > 0) entry.episode else 1).toString()) }
        var streamIdState by remember(entry) { mutableStateOf(entry.streamId) }
        var epTitleState by remember(entry) { mutableStateOf(entry.episodeTitle ?: "") }
        val defaultDur = (entry.durationMs.takeIf { it > 60_000L }
            ?: if (entry.mediaType == "movie") 110L * 60_000L else 24L * 60_000L)
        var durMs by remember(entry) { mutableStateOf(defaultDur) }
        val maxMin = (durMs / 60_000L).toInt().coerceAtLeast(1)
        var watchedMinText by remember(entry) {
            mutableStateOf(((entry.positionMs / 60_000L).toInt().coerceIn(0, maxMin)).toString())
        }
        var progressPct by remember(entry) {
            mutableFloatStateOf(
                if (durMs > 0L) ((entry.positionMs.toFloat() / durMs.toFloat()) * 100f).coerceIn(0f, 95f) else 0f,
            )
        }

        fun syncTimingFor(seasonNum: Int, episodeNum: Int, sid: String) {
            val matched = progressRows.firstOrNull { sid.isNotBlank() && it.streamId == sid }
                ?: progressRows.firstOrNull { it.season == seasonNum && it.episode == episodeNum }
            val fallbackDur = progressRows.firstOrNull { it.durationMs > 60_000L }?.durationMs
                ?: entry.durationMs.takeIf { it > 60_000L }
                ?: if (entry.mediaType == "movie") 110L * 60_000L else 24L * 60_000L
            val nextDur = matched?.durationMs?.takeIf { it > 60_000L } ?: fallbackDur
            val pos = matched?.positionMs ?: if (sid == entry.streamId) entry.positionMs else 0L
            durMs = nextDur
            val curMax = (nextDur / 60_000L).toInt().coerceAtLeast(1)
            watchedMinText = ((pos / 60_000L).toInt().coerceIn(0, curMax)).toString()
            progressPct = ((pos.toFloat() / nextDur.toFloat()) * 100f).coerceIn(0f, 95f)
        }

        LaunchedEffect(entry) {
            runCatching {
                val pRows = runCatching { repo.listMediaWatchProgress(entry.media.pluginId, entry.media.nativeId) }.getOrDefault(emptyList())
                progressRows = pRows
                if (entry.mediaType != "movie") {
                    val det = runCatching { repo.getDetails(entry.media.pluginId, entry.media.nativeId) }.getOrNull()
                    if (det != null) {
                        episodesList = det.episodes
                    }
                }
                syncTimingFor(seasonText.toIntOrNull() ?: 1, episodeText.toIntOrNull() ?: 1, streamIdState)
            }
            loadingEps = false
        }

        AlertDialog(
            onDismissRequest = { if (!savingEdit) editingCwEntry = null },
            title = {
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(VesselI18n.tr("home.edit_cw"), fontWeight = FontWeight.Bold)
                    Text(entry.title, style = MaterialTheme.typography.bodySmall, color = palette.textMuted)
                }
            },
            text = {
                Column(
                    modifier = Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    if (entry.mediaType != "movie") {
                        if (episodesList.isNotEmpty()) {
                            Box {
                                val selectedEp = episodesList.firstOrNull { it.streamId == streamIdState }
                                val epLabel = selectedEp?.let {
                                    "S${it.season} · E${it.episode}${it.title?.takeIf { t -> t.isNotBlank() }?.let { t -> " — $t" } ?: ""}"
                                } ?: "S$seasonText · E$episodeText"
                                VesselOutlinedButton(onClick = { showEpMenu = true }, modifier = Modifier.fillMaxWidth()) {
                                    Text(epLabel, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                                    Icon(Icons.Default.KeyboardArrowDown, contentDescription = null, modifier = Modifier.size(16.dp))
                                }
                                DropdownMenu(expanded = showEpMenu, onDismissRequest = { showEpMenu = false }) {
                                    episodesList.forEach { epItem ->
                                        DropdownMenuItem(
                                            text = {
                                                Text("S${epItem.season} · E${epItem.episode}${epItem.title?.takeIf { t -> t.isNotBlank() }?.let { t -> " — $t" } ?: ""}")
                                            },
                                            onClick = {
                                                showEpMenu = false
                                                streamIdState = epItem.streamId
                                                seasonText = epItem.season.toString()
                                                episodeText = epItem.episode.toString()
                                                epTitleState = epItem.title ?: ""
                                                syncTimingFor(epItem.season, epItem.episode, epItem.streamId)
                                            },
                                        )
                                    }
                                }
                            }
                        } else if (loadingEps) {
                            Text(VesselI18n.tr("common.loading"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                        }

                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                            VesselOutlinedTextField(
                                value = seasonText,
                                onValueChange = { v ->
                                    val clean = v.filter { it.isDigit() }
                                    seasonText = clean
                                    val s = clean.toIntOrNull() ?: 1
                                    val e = episodeText.toIntOrNull() ?: 1
                                    episodesList.firstOrNull { it.season == s && it.episode == e }?.let { m ->
                                        streamIdState = m.streamId
                                        epTitleState = m.title ?: ""
                                        syncTimingFor(s, e, m.streamId)
                                    }
                                },
                                label = { Text(VesselI18n.tr("home.cw_season")) },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                singleLine = true,
                                modifier = Modifier.weight(1f).vesselDpadInputEscape(isTextEmpty = seasonText.isEmpty()),
                            )
                            VesselOutlinedTextField(
                                value = episodeText,
                                onValueChange = { v ->
                                    val clean = v.filter { it.isDigit() }
                                    episodeText = clean
                                    val s = seasonText.toIntOrNull() ?: 1
                                    val e = clean.toIntOrNull() ?: 1
                                    episodesList.firstOrNull { it.season == s && it.episode == e }?.let { m ->
                                        streamIdState = m.streamId
                                        epTitleState = m.title ?: ""
                                        syncTimingFor(s, e, m.streamId)
                                    }
                                },
                                label = { Text(VesselI18n.tr("home.cw_ep_num")) },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                singleLine = true,
                                modifier = Modifier.weight(1f).vesselDpadInputEscape(isTextEmpty = episodeText.isEmpty()),
                            )
                        }
                    }

                    val curDurMin = (durMs / 60_000L).toInt().coerceAtLeast(1)
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text(VesselI18n.tr("home.cw_progress"), style = MaterialTheme.typography.labelMedium, color = palette.textMuted)
                            Text("${progressPct.toInt()}%", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.Bold)
                        }
                        Slider(
                            value = progressPct,
                            onValueChange = { pct ->
                                progressPct = pct
                                watchedMinText = ((curDurMin * pct) / 100f).toInt().toString()
                            },
                            valueRange = 0f..95f,
                        )
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                        VesselOutlinedTextField(
                            value = watchedMinText,
                            onValueChange = { v ->
                                val clean = v.filter { it.isDigit() }
                                watchedMinText = clean
                                val mins = (clean.toIntOrNull() ?: 0).coerceIn(0, curDurMin)
                                progressPct = ((mins.toFloat() / curDurMin.toFloat()) * 100f).coerceIn(0f, 99f)
                            },
                            label = { Text(VesselI18n.tr("home.cw_watched_min")) },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f).vesselDpadInputEscape(isTextEmpty = watchedMinText.isEmpty()),
                        )
                        Column(modifier = Modifier.weight(1f)) {
                            Text(VesselI18n.tr("home.cw_duration_min"), style = MaterialTheme.typography.labelSmall, color = palette.textMuted)
                            Text("$curDurMin min", style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        VesselTextButton(
                            enabled = !savingEdit,
                            onClick = {
                                scope.launch {
                                    repo.deleteWatchProgress(entry.media.pluginId, entry.media.nativeId)
                                    editingCwEntry = null
                                    loadHome()
                                }
                            },
                        ) {
                            Icon(Icons.Default.Delete, contentDescription = null, tint = palette.danger, modifier = Modifier.size(15.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(VesselI18n.tr("common.remove"), color = palette.danger)
                        }
                        VesselOutlinedButton(
                            enabled = !savingEdit,
                            onClick = {
                                savingEdit = true
                                scope.launch {
                                    val s = if (entry.mediaType == "movie") entry.season else (seasonText.toIntOrNull() ?: 1).coerceAtLeast(0)
                                    val e = if (entry.mediaType == "movie") entry.episode else (episodeText.toIntOrNull() ?: 1).coerceAtLeast(0)
                                    val matchedEp = episodesList.firstOrNull { it.streamId == streamIdState || (it.season == s && it.episode == e) }
                                    val safeDur = durMs.coerceAtLeast(60_000L)
                                    runCatching {
                                        repo.recordProgress(
                                            PlaybackProgress(
                                                media = entry.media,
                                                streamId = matchedEp?.streamId ?: streamIdState,
                                                canonicalId = entry.canonicalId,
                                                season = s,
                                                episode = e,
                                                absoluteEpisode = matchedEp?.absoluteEpisode ?: entry.absoluteEpisode,
                                                title = entry.title,
                                                posterUrl = entry.poster,
                                                backdropUrl = entry.backdrop,
                                                episodeTitle = matchedEp?.title ?: epTitleState.ifBlank { entry.episodeTitle },
                                                positionMs = safeDur,
                                                durationMs = safeDur,
                                                completed = true,
                                            ),
                                        )
                                        loadHome()
                                        editingCwEntry = null
                                    }.onFailure { repo.notifyError(it, "Failed to update progress") }
                                    savingEdit = false
                                }
                            },
                        ) {
                            Icon(Icons.Default.CheckCircle, contentDescription = null, modifier = Modifier.size(14.dp))
                            Spacer(Modifier.width(4.dp))
                            Text(VesselI18n.tr("home.cw_mark_done"), style = MaterialTheme.typography.labelSmall)
                        }
                    }
                }
            },
            confirmButton = {
                VesselButton(
                    enabled = !savingEdit,
                    onClick = {
                        savingEdit = true
                        scope.launch {
                            val s = if (entry.mediaType == "movie") entry.season else (seasonText.toIntOrNull() ?: 1).coerceAtLeast(0)
                            val e = if (entry.mediaType == "movie") entry.episode else (episodeText.toIntOrNull() ?: 1).coerceAtLeast(0)
                            val matchedEp = episodesList.firstOrNull { it.streamId == streamIdState || (it.season == s && it.episode == e) }
                            val safeDur = durMs.coerceAtLeast(60_000L)
                            val posMs = ((safeDur * progressPct) / 100f).toLong().coerceIn(10_000L, safeDur)
                            runCatching {
                                repo.recordProgress(
                                    PlaybackProgress(
                                        media = entry.media,
                                        streamId = matchedEp?.streamId ?: streamIdState,
                                        canonicalId = entry.canonicalId,
                                        season = s,
                                        episode = e,
                                        absoluteEpisode = matchedEp?.absoluteEpisode ?: entry.absoluteEpisode,
                                        title = entry.title,
                                        posterUrl = entry.poster,
                                        backdropUrl = entry.backdrop,
                                        episodeTitle = matchedEp?.title ?: epTitleState.ifBlank { entry.episodeTitle },
                                        positionMs = posMs,
                                        durationMs = safeDur,
                                        completed = false,
                                    ),
                                )
                                loadHome()
                                editingCwEntry = null
                                repo.notify("${entry.title} · ${VesselI18n.tr("common.save")}")
                            }.onFailure { repo.notifyError(it, "Failed to update progress") }
                            savingEdit = false
                        }
                    },
                ) {
                    Text(VesselI18n.tr("common.save"))
                }
            },
            dismissButton = {
                VesselTextButton(enabled = !savingEdit, onClick = { editingCwEntry = null }) {
                    Text(VesselI18n.tr("common.cancel"))
                }
            },
        )
    }
}
