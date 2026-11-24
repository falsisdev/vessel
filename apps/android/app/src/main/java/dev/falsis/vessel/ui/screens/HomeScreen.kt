package dev.falsis.vessel.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import dev.falsis.vessel.data.ContinueWatchingEntry
import dev.falsis.vessel.data.HomeShelfResult
import dev.falsis.vessel.data.LibraryItem
import dev.falsis.vessel.data.MediaDetail
import dev.falsis.vessel.data.MediaRef
import dev.falsis.vessel.data.MediaSummary
import dev.falsis.vessel.data.VesselRepository
import dev.falsis.vessel.ui.components.ContinueWatchingCard
import dev.falsis.vessel.ui.components.MediaPosterCard
import dev.falsis.vessel.ui.components.VesselLogoMark
import dev.falsis.vessel.ui.i18n.VesselI18n
import dev.falsis.vessel.ui.theme.LocalVesselPalette
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import android.util.Log

private data class SpotlightHeroItem(
    val summary: MediaSummary,
    val pluginId: String,
    val pluginName: String,
)

private fun resolveHomeGroupName(p: dev.falsis.vessel.data.PluginListEntry): String {
    val rid = p.repoId?.trim()
    if (!rid.isNullOrEmpty()) return rid
    if (
        "raw.githubusercontent.com" in p.allowedDomains &&
        (p.manifest.author == "falsisdev" ||
            p.manifest.author == "metemel" ||
            p.manifest.id.startsWith("anthology") ||
            p.manifest.id == "m3u.anthology.addon")
    ) {
        return "Anthology"
    }
    if (p.manifest.id.startsWith("community.anthology")) return "Anthology"
    if (p.sourceType == "dev_dir") return "Local Dev Plugins"
    return "Standalone Plugins"
}

private fun pickDefaultStarterPlugins(enabled: List<dev.falsis.vessel.data.PluginListEntry>): List<String> {
    // Same smart curated <= 5 VOD/anime starter selection as Desktop HomeView.vue
    val vodEnabled = enabled.filter { p ->
        val caps = p.manifest.capabilities
        !(caps.contains("live_tv") && !caps.contains("vod"))
    }
    val preferred = listOf("anizium", "diziwatch", "yabancidizi", "animecix", "seicode")
    val vodIds = vodEnabled.map { it.manifest.id }.toSet()
    val picked = mutableListOf<String>()
    for (id in preferred) {
        if (id in vodIds && picked.size < 5) picked += id
    }
    for (p in vodEnabled) {
        if (p.manifest.id !in picked && picked.size < 5) picked += p.manifest.id
    }
    if (picked.isEmpty()) {
        enabled.take(5).mapTo(picked) { it.manifest.id }
    }
    return picked
}

@Composable
fun HomeScreen(
    repo: VesselRepository,
    onOpenDetails: (pluginId: String, nativeId: String) -> Unit,
    onResumeEntry: (ContinueWatchingEntry) -> Unit,
    onOpenExtensions: () -> Unit,
) {
    val palette = LocalVesselPalette.current
    val scope = rememberCoroutineScope()
    val isReady by repo.isReady.collectAsState()
    val initError by repo.initError.collectAsState()
    val plugins by repo.plugins.collectAsState()
    val prefs by repo.uiPreferences.collectAsState()

    var loading by remember { mutableStateOf(true) }
    var shelves by remember { mutableStateOf<List<HomeShelfResult>>(emptyList()) }
    var continueWatching by remember { mutableStateOf<List<ContinueWatchingEntry>>(emptyList()) }
    var showFeedsModal by remember { mutableStateOf(false) }
    val heroDetails = remember { mutableStateMapOf<String, MediaDetail>() }

    val enabledPlugins = remember(plugins) { plugins.filter { it.enabled } }
    val enabledPluginIds = remember(enabledPlugins) { enabledPlugins.map { it.manifest.id } }
    val effectiveHomeIds = remember(enabledPlugins, prefs.home.selectedPlugins) {
        val enabledSet = enabledPlugins.map { it.manifest.id }.toSet()
        val explicit = prefs.home.selectedPlugins?.filter { it in enabledSet }.orEmpty()
        if (explicit.isNotEmpty()) explicit else pickDefaultStarterPlugins(enabledPlugins)
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
        runCatching {
            shelves = if (effectiveHomeIds.isEmpty()) emptyList() else repo.getHomeFeeds(effectiveHomeIds)
        }.onFailure { Log.e("VesselHome", "getHomeFeeds failed", it) }
        loading = false
    }

    LaunchedEffect(isReady, effectiveHomeIds) {
        if (isReady) loadHome()
    }

    val spotlightPool: List<SpotlightHeroItem> = remember(shelves, prefs.home.heroPlugins, enabledPluginIds) {
        val heroFilter = prefs.home.heroPlugins?.filter { it in enabledPluginIds }?.takeIf { it.isNotEmpty() }?.toSet()
        val filteredShelves = if (heroFilter != null) {
            val matched = shelves.filter { it.pluginId in heroFilter }
            if (matched.isNotEmpty()) matched else shelves
        } else {
            shelves
        }
        val allCandidates = filteredShelves.flatMap { shelf ->
            shelf.sections.flatMap { sec ->
                sec.items.map { item ->
                    SpotlightHeroItem(
                        summary = item,
                        pluginId = item.pluginId ?: shelf.pluginId,
                        pluginName = shelf.pluginName,
                    )
                }
            }
        }
        val withPoster = allCandidates.filter { !it.summary.poster.isNullOrBlank() }
        val vodPreferred = withPoster.filter { it.summary.type != "live" }
        val pool = when {
            vodPreferred.isNotEmpty() -> vodPreferred
            withPoster.isNotEmpty() -> withPoster
            else -> allCandidates
        }
        pool.distinctBy { "${it.pluginId}:${it.summary.nativeId}" }.take(6)
    }

    val hasEnabledPlugins = enabledPluginIds.isNotEmpty()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(palette.bgBase),
        contentPadding = PaddingValues(bottom = 24.dp),
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
                        Text("Native Engine Setup Required", fontWeight = FontWeight.Bold, color = palette.danger)
                        Text(err, style = MaterialTheme.typography.bodySmall, color = palette.textPrimary)
                    }
                }
            }
        }

        // CloudStream-style Full-Bleed Hero Stage (replaces brand header once plugins exist)
        if (hasEnabledPlugins && prefs.home.heroBanner != "hidden") {
            val heroHeight = if (prefs.home.heroBanner == "compact") 300.dp else 430.dp
            item(key = "spotlight-hero") {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(heroHeight)
                        .background(palette.bgSurface),
                ) {
                    if (spotlightPool.isNotEmpty()) {
                        val pagerState = rememberPagerState(pageCount = { spotlightPool.size })

                        LaunchedEffect(spotlightPool.size, pagerState.currentPage, pagerState.isScrollInProgress) {
                            if (spotlightPool.size > 1 && !pagerState.isScrollInProgress) {
                                delay(6500)
                                val next = (pagerState.currentPage + 1) % spotlightPool.size
                                pagerState.animateScrollToPage(next)
                            }
                        }

                        HorizontalPager(
                            state = pagerState,
                            modifier = Modifier.fillMaxSize(),
                        ) { page ->
                            val hero = spotlightPool[page]
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

                            Box(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .clickable { onOpenDetails(hero.pluginId, hero.summary.nativeId) },
                            ) {
                                if (!fallbackUrl.isNullOrBlank()) {
                                    AsyncImage(
                                        model = fallbackUrl,
                                        contentDescription = hero.summary.title,
                                        contentScale = ContentScale.Crop,
                                        modifier = Modifier.fillMaxSize(),
                                    )
                                }
                                if (!backdropUrl.isNullOrBlank()) {
                                    AsyncImage(
                                        model = backdropUrl,
                                        contentDescription = hero.summary.title,
                                        contentScale = ContentScale.Crop,
                                        modifier = Modifier.fillMaxSize(),
                                    )
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
                                        .height(100.dp)
                                        .align(Alignment.TopCenter)
                                        .background(
                                            Brush.verticalGradient(
                                                listOf(Color.Black.copy(alpha = 0.62f), Color.Transparent),
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
                                                0.38f to Color.Black.copy(alpha = 0.25f),
                                                0.74f to palette.bgBase.copy(alpha = 0.82f),
                                                1.0f to palette.bgBase,
                                            ),
                                        ),
                                )

                                Column(
                                    modifier = Modifier
                                        .align(Alignment.BottomStart)
                                        .fillMaxWidth()
                                        .padding(start = 18.dp, end = 18.dp, bottom = if (spotlightPool.size > 1) 30.dp else 16.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    ) {
                                        Surface(
                                            color = palette.accent.copy(alpha = 0.9f),
                                            shape = RoundedCornerShape(6.dp),
                                        ) {
                                            Text(
                                                text = if (hero.summary.isAnime) "ANIME" else hero.summary.type.uppercase(),
                                                style = MaterialTheme.typography.labelSmall,
                                                fontWeight = FontWeight.Bold,
                                                color = Color(0xFF11111B),
                                                modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
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
                                                    modifier = Modifier.size(14.dp),
                                                )
                                                Text(
                                                    text = String.format("%.1f", rating),
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
                                                modifier = Modifier.padding(horizontal = 7.dp, vertical = 2.dp),
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

                                    Row(
                                        modifier = Modifier.padding(top = 4.dp),
                                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                                    ) {
                                        Button(
                                            onClick = { onOpenDetails(hero.pluginId, hero.summary.nativeId) },
                                            shape = RoundedCornerShape(50),
                                            colors = ButtonDefaults.buttonColors(
                                                containerColor = palette.accent,
                                                contentColor = Color(0xFF11111B),
                                            ),
                                        ) {
                                            Icon(Icons.Default.PlayArrow, contentDescription = null, modifier = Modifier.size(18.dp))
                                            Spacer(Modifier.width(6.dp))
                                            Text(VesselI18n.tr("home.watch_now"), fontWeight = FontWeight.Bold)
                                        }
                                        OutlinedButton(
                                            onClick = { onOpenDetails(hero.pluginId, hero.summary.nativeId) },
                                            shape = RoundedCornerShape(50),
                                            border = BorderStroke(1.dp, Color.White.copy(alpha = 0.35f)),
                                            colors = ButtonDefaults.outlinedButtonColors(
                                                containerColor = Color.Black.copy(alpha = 0.38f),
                                                contentColor = Color.White,
                                            ),
                                        ) {
                                            Icon(Icons.Default.Info, contentDescription = null, modifier = Modifier.size(17.dp))
                                            Spacer(Modifier.width(6.dp))
                                            Text(VesselI18n.tr("home.more_info"))
                                        }
                                    }
                                }
                            }
                        }

                        if (spotlightPool.size > 1) {
                            Row(
                                modifier = Modifier
                                    .align(Alignment.BottomCenter)
                                    .padding(bottom = 10.dp),
                                horizontalArrangement = Arrangement.spacedBy(6.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                repeat(spotlightPool.size) { index ->
                                    val active = pagerState.currentPage == index
                                    Box(
                                        modifier = Modifier
                                            .height(5.dp)
                                            .width(if (active) 20.dp else 6.dp)
                                            .clip(CircleShape)
                                            .background(
                                                if (active) palette.accent else Color.White.copy(alpha = 0.38f),
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
                            CircularProgressIndicator(color = palette.accent)
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
                                verticalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Text(
                                    text = VesselI18n.tr("home.empty_selected"),
                                    color = palette.textMuted,
                                    textAlign = TextAlign.Center,
                                )
                                Button(onClick = { showFeedsModal = true }) {
                                    Text(VesselI18n.tr("home.choose_feeds"))
                                }
                            }
                        }
                    }

                    // Floating Source & Refresh controls over top-right of Hero
                    Row(
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(horizontal = 12.dp, vertical = 10.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Surface(
                            shape = CircleShape,
                            color = Color.Black.copy(alpha = 0.45f),
                        ) {
                            IconButton(onClick = { showFeedsModal = true }) {
                                Icon(Icons.Default.Tune, contentDescription = VesselI18n.tr("home.choose_feeds"), tint = Color.White)
                            }
                        }
                        Surface(
                            shape = CircleShape,
                            color = Color.Black.copy(alpha = 0.45f),
                        ) {
                            IconButton(onClick = { scope.launch { loadHome(forceRefresh = true) } }) {
                                Icon(Icons.Default.Refresh, contentDescription = VesselI18n.tr("common.refresh"), tint = Color.White)
                            }
                        }
                    }
                }
            }
        } else if (hasEnabledPlugins) {
            item {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
                    horizontalArrangement = Arrangement.End,
                ) {
                    IconButton(onClick = { showFeedsModal = true }) {
                        Icon(Icons.Default.Tune, contentDescription = VesselI18n.tr("home.choose_feeds"))
                    }
                    IconButton(onClick = { scope.launch { loadHome(forceRefresh = true) } }) {
                        Icon(Icons.Default.Refresh, contentDescription = VesselI18n.tr("common.refresh"))
                    }
                }
            }
        }

        // Continue Watching Rail
        if (prefs.home.showContinueWatching && continueWatching.isNotEmpty()) {
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
                        items(continueWatching, key = { "${it.progress.media.pluginId}:${it.progress.media.nativeId}" }) { entry ->
                            ContinueWatchingCard(
                                entry = entry,
                                onResume = { onResumeEntry(entry) },
                                onGoToDetails = {
                                    onOpenDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
                                },
                                onEdit = {
                                    onOpenDetails(entry.progress.media.pluginId, entry.progress.media.nativeId)
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
                    Button(onClick = onOpenExtensions) {
                        Text(VesselI18n.tr("nav.extensions"))
                    }
                }
            }
        } else if (loading && shelves.isEmpty()) {
            item {
                Box(modifier = Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }
        } else {
            shelves.forEach { result ->
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
                                                    repo.notify("${item.title} → $status")
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
        val grouped = remember(enabledPlugins) {
            enabledPlugins.groupBy(::resolveHomeGroupName).toList()
        }
        var draftSelection by remember(showFeedsModal) { mutableStateOf(effectiveHomeIds.toSet()) }
        var draftHeroPlugins by remember(showFeedsModal) {
            mutableStateOf(prefs.home.heroPlugins?.filter { it in enabledPluginIds }?.toSet() ?: emptySet())
        }
        AlertDialog(
            onDismissRequest = { showFeedsModal = false },
            title = { Text(VesselI18n.tr("home.modal_title")) },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(420.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        text = VesselI18n.tr("home.modal_subtitle"),
                        style = MaterialTheme.typography.bodySmall,
                        color = palette.textMuted,
                    )
                    grouped.forEach { (groupName, groupItems) ->
                        val groupIds = groupItems.map { it.manifest.id }.toSet()
                        val allGroupSelected = groupIds.all { it in draftSelection }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(groupName, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelLarge)
                            TextButton(onClick = {
                                draftSelection = if (allGroupSelected) draftSelection - groupIds else draftSelection + groupIds
                            }) {
                                Text(
                                    if (allGroupSelected) VesselI18n.tr("common.clear", "Clear") else VesselI18n.tr("common.all", "All"),
                                    style = MaterialTheme.typography.labelSmall,
                                )
                            }
                        }
                        groupItems.forEach { entry ->
                            val pid = entry.manifest.id
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        draftSelection = if (pid in draftSelection) draftSelection - pid else draftSelection + pid
                                    },
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Checkbox(
                                    checked = pid in draftSelection,
                                    onCheckedChange = { checked ->
                                        draftSelection = if (checked) draftSelection + pid else draftSelection - pid
                                    },
                                )
                                Text(entry.manifest.name, modifier = Modifier.weight(1f))
                            }
                        }
                        HorizontalDivider(color = palette.border.copy(alpha = 0.35f))
                    }

                    Text(
                        text = VesselI18n.tr("home.hero_source_title", "Spotlight Hero Source"),
                        fontWeight = FontWeight.Bold,
                        style = MaterialTheme.typography.labelLarge,
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        androidx.compose.material3.FilterChip(
                            selected = draftHeroPlugins.isEmpty(),
                            onClick = { draftHeroPlugins = emptySet() },
                            label = { Text(VesselI18n.tr("home.hero_source_auto", "Auto")) },
                        )
                        enabledPlugins.filter { it.manifest.id in draftSelection }.forEach { entry ->
                            val pid = entry.manifest.id
                            androidx.compose.material3.FilterChip(
                                selected = pid in draftHeroPlugins,
                                onClick = {
                                    draftHeroPlugins = if (pid in draftHeroPlugins) draftHeroPlugins - pid else draftHeroPlugins + pid
                                },
                                label = { Text(entry.manifest.name) },
                            )
                        }
                    }
                }
            },
            confirmButton = {
                Button(onClick = {
                    showFeedsModal = false
                    val nextSelected = draftSelection.toList().takeIf { it.isNotEmpty() }
                    val nextHero = draftHeroPlugins.toList().takeIf { it.isNotEmpty() }
                    scope.launch {
                        repo.saveUiPreferences(
                            prefs.copy(
                                home = prefs.home.copy(
                                    selectedPlugins = nextSelected,
                                    heroPlugins = nextHero,
                                ),
                            ),
                        )
                    }
                }) {
                    Text(VesselI18n.tr("common.save"))
                }
            },
            dismissButton = {
                TextButton(onClick = { showFeedsModal = false }) {
                    Text(VesselI18n.tr("common.cancel"))
                }
            },
        )
    }
}
