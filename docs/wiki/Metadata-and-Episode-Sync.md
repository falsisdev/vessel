> 🌐 **English** · [Wiki Home](Home)

# Metadata Enrichment & Episode Coordinate Sync

Vessel separates **self-contained playback** from **optional rich metadata (`vessel-metadata`)**. Plugins never have to integrate external APIs—the Rust engine enriches titles cleanly in the background and caches results in SQLite (`meta2`).

---

## 1. Metadata Providers

| Provider | API Key Required? | Supplied Data |
| :--- | :--- | :--- |
| **AniList GraphQL** | **No** (zero-key) | Canonical anime title, poster, banner, genres, rating, status, synonyms, relations |
| **Cinemeta (IMDb)** | **No** (zero-key) | Movie & series metadata, IMDb `tt...` IDs, cast/backdrop art |
| **AniZip (`api.ani.zip`)** | **No** (zero-key) | Episode stills (thumbnails), localized episode titles, descriptions, and cross-ID mappings (`anilist_id` ↔ `mal_id` / `imdb_id` / `tmdb_id`) |
| **TMDB** | **Optional** (user API key in Settings) | Enhanced high-res posters, backdrops, TMDB IDs, and rich multilingual episode overviews |

---

## 2. Episode Coordinate Translation (`season, episode ↔ absolute_episode`)

Many anime sources number long-running series continuously (`Episode 50`, `Episode 142`), while TMDB/Cinemeta organize them into seasons (`Season 1 Episode 50` vs `Season 2 Episode 1`), or vice versa:
- **`vessel-metadata/src/coordinates.rs`** aligns flat single-season episode lists and translates `(season, episode) ↔ absolute_episode` without ever mutating the plugin's authoritative `streamId`.
- **Immutable Plugin Playback:** A plugin's `streamId` always stays untouched so playback resolves 100% faithfully, while watch progress persists **dual coordinates** (`season`, `episode`, and `absoluteEpisode`). That means switching to another plugin for the exact same show still resumes cleanly!
- **Real Episode Title Restoration:** Generic localized placeholders returned by scrapers (e.g. `1. Sezon 4. Bölüm` or `Episode 4`) are replaced automatically with real episode titles from enriched metadata when available, while preserving genuine custom titles returned by plugins.

---

## 3. Manual **Fix Match** Overrides

If title matching picks the wrong entry (e.g., remakes, spin-offs, or regional title collisions):
1. Open the item's **Details** page and click **Fix Match**.
2. Enter a canonical ID directly:
   - `anilist:918` (AniList)
   - `tmdb:tv:1399` or `tmdb:movie:603` (TMDB)
   - `imdb:tt0944947` (IMDb / Cinemeta)
3. Or toggle **Disable external metadata for this title** to revert exclusively to raw plugin data.

Overrides are stored per `(pluginId, nativeId)` in SQLite and survive restarts and backups.

---

## 4. Strict Privacy Mode (Zero External Queries)

Want plugins to talk **only** to their own declared domains with zero metadata or skip-DB calls to AniList / Cinemeta / AniZip / TMDB / TheIntroDB / IntroDB / AniSkip?
- Turn off **Enable Background Metadata Enrichment** in **Settings → Playback & Privacy**.
- Vessel still runs local progress tracking, Continue Watching, Smart Proxy, and offline downloads 100% locally.
