> 🌐 **English** · [Türkçe](Desktop-App-User-Guide-tr) · [Wiki Home](Home)

# Desktop App User Guide

Vessel's desktop interface (`apps/desktop`) combines **Tauri v2** with **Vue 3** to provide a responsive, cinema-grade media center across **Home**, **Search**, **Library**, **Live TV**, **Downloads**, **Extensions**, and **Settings**.

---

## 1. Navigation & Window Flow

- **Animated Splash Reveal:** Opening Vessel plays a custom animated logo transition straight into your home showcase.
- **Smooth Page Fades:** Switching views triggers subtle `page-fade` cross-transitions while preserving snappy responsiveness (respecting `reducedMotion`).
- **Modular Navigation:** Switch between **Floating Bottom Dock**, **Expanded Sidebar**, **Pinned Sidebar**, **Collapsed Icon Rail**, **Top Navbar**, or **Hidden** under **Settings → Modular UI Preferences**.

---

## 2. Home (`HomeView`)

### Spotlight Hero Banner
- Full-bleed crossfading showcase with subtle smooth **zoom animation** and automatic slide advancement, displaying backdrops, year/rating tags, synopsis, **Watch Now**, and **More Info** buttons.
- **Watch Now** vs **More Info:** **Watch Now** resolves and starts playback directly on Home (or opens an inline stream picker dialog when **Always Ask Source** is active or extraction/multiple streams require choosing) without redirecting to Details, while **More Info** opens the full Media Details view.
- Customize banner presentation (`Full`, `Compact`, or `Hidden`) in Settings and pick which plugins power the Hero banner inside **Choose Home Feeds → Spotlight Hero Source** (`Auto` or specific pinned plugins).

### Feed Selection & Shelf Ordering
- Click **Feeds** (or **Choose Home Feeds**) on Home to choose which active plugins display shelves on the Home screen. *(Live TV plugins are isolated on the dedicated **Live TV** tab rather than cluttering Home.)*
- Plugins unchecked from Home remain fast and active in **Global Search**.
- Drag handles inside **Active Home Shelf Order** reorder your shelves from top to bottom.

### Smart Continue Watching
Vessel automatically tracks dual-coordinate playback progress (`(season, episode)` + `absoluteEpisode`), ordered with your **most recently watched entry on the left**:
- **Movies:** Displays movie title without redundant season/episode badges. Once watched past the completion threshold (default `90%`), completed movies **automatically leave** Continue Watching while remaining marked as watched in your SQLite database.
- **Series & Anime:** Finishing an episode automatically removes the completed episode and surfaces the **Next Episode (`Up Next`)** (powered by locally persisted episode snapshots). Finishing the **series finale** automatically removes the show from Continue Watching.
- **Uniform Cards & Truncation:** All Continue Watching cards share a uniform `16:9` footprint; long show/episode labels truncate cleanly with ellipsis (`...`).
- **Context Menu (`Right-Click` or `Long-Press`):** Open a quick menu directly from any Continue Watching card with **Go to Details** and edit shortcuts.
- **Manual Continue Watching Editor (Desktop & Android):** Click the **Edit (Pencil)** icon on a Continue Watching card to switch season/episode or adjust watched progress percentage/minutes manually; **Total Duration** updates automatically (read-only) from the selected episode's known duration, or click **Mark Finished / Next** / **Remove** to update immediately.

### Quick Watch Status on Cards
Click the subtle top-right **Quick Watch Status** badge on **Home** or **Search** poster cards without visiting the Details page:
- **Watching**
- **Plan to Watch**
- **Completed**
- **On Hold**
- **Dropped**

---

## 3. Global Search (`Cmd/Ctrl + K`)

- **Parallel Streaming Search:** Searches all enabled plugins simultaneously and streams results onto the grid as each provider responds (canceling in-flight queries automatically if you edit your search).
- **Type Filters:** Filter by **All**, **Movies**, **Series**, or **Anime**.
- **Single-Plugin Filter:** Filter results down to a specific installed extension alongside the type filter.
- **Instant Quick Watch Status:** Update library status directly from search result cards.

---

## 4. Media Details (`DetailsView`)

- **Atmospheric Backdrop & Metadata:** Displays full-bleed backdrop art, Genres, Year, Rating, Synopsis, and External ID badges (`AniList`, `TMDB`, `IMDb`, `MAL`) when enriched.
- **Contextual CTA:** Automatically shows **Resume (`S·E`)** when saved watch progress exists, or **Watch** otherwise.
- **Top-Right `⋮` Menu (`Fix Match` & `Include Streams From Other Sources`):**
  - **Fix Match:** Search directly across **TMDB, IMDb, Simkl, AniList, MyAnimeList, and Kitsu** by title, bind a manual canonical ID fallback, or toggle **Disable external metadata for this title**.
  - **Include Streams From Other Sources:** Link additional installed plugins directly to the current Details page (persisted locally on your device and editable anytime) so streams from linked extensions are queried and merged seamlessly without leaving your preferred source layout.
- **Episode View Modes:** Switch between **Grid Cards** (with AniZip stills & titles), **Detailed List**, and **Compact Rows**.
- **Per-Episode Progress & Watched Badges:** Each episode card renders its watched progress bar underneath and a green **Watched** badge once completed. Real metadata episode titles replace generic placeholders (e.g. `1. Sezon 3. Bölüm`) whenever available.
- **Source Resolver Sheet:** Clicking an episode or **Streams** opens a live stream picker showing stream labels, quality, language, audio classification (`Sub`, `Dub`, `Original`), and direct **Download** buttons alongside playback.

---

## 5. Library (`LibraryView`)

- Filter saved titles across all 5 statuses (**All**, **Watching**, **Plan to Watch**, **Completed**, **On Hold**, **Dropped**).
- Hover any library card to change status quickly or click the dedicated **top-left Remove (`×`)** button to remove an item cleanly without overlapping top-right status controls.

---

## 6. Portable Backups (`.backup.vessel`)

From **Settings → Portable Backup**:
- **Export Backup:** Exports library, watch progress, plugin list, custom UI preferences, and metadata overrides to `.backup.vessel` JSON. *(Stored plugin secrets/API keys are excluded by default for safe sharing unless explicitly toggled on.)*
- **Import (Smart Merge):** Merges imported state into your local SQLite DB while preserving newer watch timestamps.
- **Import (Reset & Replace):** Replaces current local state cleanly from a snapshot.
