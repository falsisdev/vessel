> 🌐 **English** · [Türkçe](Player-and-Downloads-tr) · [Wiki Home](Home)

# Offline Downloads & Queue Management

Vessel includes a full-featured **Rust offline download engine (`vessel-downloader`)** paired with a **4-level hierarchical offline library** in `DownloadsView`.

---

## 1. Concurrent Download Engine (`vessel-downloader`)

- **Direct Transfers (MP4 / MKV / WebM):** Probes upstream `Accept-Ranges: bytes` + `Content-Length` support and splits large files into **concurrent HTTP `Range` workers** with atomic chunk assembly and resume support. Falls back cleanly to streaming transfer when `Range` isn't supported.
- **HLS Streams (`.m3u8`) with 8-Worker Parallelism:**
  - Parses variant & master playlists automatically.
  - Downloads video segments concurrently across **8 parallel Tokio workers** and decrypts **`AES-128`** encrypted `#EXT-X-KEY` segments locally.
  - Automatically downloads and bundles **alternate `EXT-X-MEDIA` audio tracks** and **HLS subtitle playlists** into the local offline `.m3u8` directory.
- **External Subtitle Bundling:** When initiating a download, associated external `.srt` / `.vtt` / `.ass` / `.ssa` subtitle tracks are downloaded concurrently alongside the video and registered in SQLite (`localSubtitles`).
- **Custom Download Directory:** Configure where offline files live under **Settings → Download Location**.

---

## 2. Instant Pause, Delete & Drag-and-Drop Queue

- **Cooperative Instant Cancellation (`tokio::select! { biased; ... }` + `join_set.abort_all()`):** Pausing or deleting an active download immediately aborts all in-flight HTTP chunk/segment requests instead of waiting for slow sockets to drain, emits an authoritative `DownloadStatus::Paused` snapshot, removes partial folders on delete (with transient OS-lock retry), and immediately dispatches the next queued job.
- **Drag-and-Drop Queue Ordering:** Inside **Download Queue**, grab any queued, active, paused, or failed item's drag handle to reorder download priority (`queueOrder` persisted in SQLite).
- **Single-Click Pause / Resume / Delete:** Protected by optimistic UI snapshot updates and per-item action locks (`busyIds`).

---

## 3. 4-Level Hierarchical Offline Library

Downloaded content is organized automatically into a collapsible multi-level tree inside **Downloads**:

- **For Series & Anime (4 Levels):**
  1. **Level 1 — Content (Show):** Title, total seasons/episodes/streams, aggregated downloaded size, active transfer badge, and **Watched** checkmark if all downloaded episodes are completed.
  2. **Level 2 — Season:** Expand a show to see downloaded seasons (`Season 1`, `Season 2`, etc.), episode & stream counts, season size, and season completion status.
  3. **Level 3 — Episode:** Expand a season to see individual downloaded episodes with watch progress bar underneath and green **Watched** badge when finished.
  4. **Level 4 — Downloaded Streams:** Expand an episode to see **every downloaded source separately** labeled by its `streamLabel` (e.g. `1080p • Server A` vs `720p • Backup`). If two streams were downloaded for the same episode, both appear grouped cleanly under that episode! Clicking a stream launches offline playback immediately via `http://127.0.0.1:<port>/local/...`.

- **For Movies (Direct 2-Level Grouping):**
  - Movies expand directly from **Content (Movie)** → **Downloaded Streams** (`streamLabel`), without redundant season/episode folders.

- **Offline Continue Watching Rail:** Top section of **Downloads** surfaces unfinished offline downloads with remaining time, progress bars, and one-click resume directly from local storage—fully functional without an internet connection!
