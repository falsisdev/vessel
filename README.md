<p align="center">
  <img src="assets/vessel-logo.png" alt="Vessel Logo" width="144" height="144" />
</p>

<h1 align="center">Vessel</h1>

<p align="center">
  <em>Most <strong>ships</strong> dread <strong>Rust</strong>. This <strong>cargo vessel</strong> runs on it — <strong>shipped</strong> with <strong>Cargo</strong>, engineered not to <strong>rust</strong>, <strong>docking</strong> plugins by <strong>manifest</strong> at local <strong>ports</strong>.</em>
</p>

**Vessel** is a local-first, cross-platform streaming and media center built from the ground up in **Rust**, **Tauri v2**, and **Vue 3 / TypeScript**.

> **Project Status:** Core Engine (**Milestones 0–5**) and the **macOS Desktop MVP (`apps/desktop`)** are implemented and running. Currently focusing on real-world plugins and **v1.0 Desktop Packaging (Milestone 6.1)**, followed by **Android / Android TV via UniFFI (Milestone 6.2)**.
> The previous Go + SwiftUI prototype is archived on `archive/v1-go` (`v1-go-archive` tag) and `archive/v1-go-ui-polish` (`v1-go-ui-polish-archive` tag).

---

## Core Principles

- **Local-First & Zero Server Cost:** Plugin execution, library state, watch progress, metadata caching, offline downloads, and the local HTTP/HLS proxy (`127.0.0.1`) all run directly on the user's device. Zero telemetry.
- **Replaceable Shell, Permanent Brain:** All business logic lives inside UI-agnostic Rust crates (`vessel-engine` and its workspace crates), keeping UI shells thin and portable across Desktop, Android, and iOS.
- **Self-Contained Plugins + Optional Metadata Enrichment:** Community plugins (TypeScript/JavaScript via `rquickjs` + `oxc`) never depend on external IDs (TMDB/IMDb/AniList) to list episodes or resolve streams, while an optional enrichment layer adds rich posters, backdrops, synopses, and cross-provider episode coordinate mapping.
- **CloudStream / Nuvio-Grade UX:** Source resolution before playback, full-screen in-app player controls, Continue Watching persistence, Fix Match manual ID overrides, built-in IPTV + XMLTV EPG, and offline HLS/range downloads.

---

## Implemented Features

### Core Engine & Runtime (Rust)
- **Isolated JS/TS Plugin Runtime (`vessel-plugin-rt`):** Powered by `rquickjs` + `oxc` TypeScript stripping with SQLite compile caching, ES6 module loading (Dev Mode directories & `.plugin.vessel` zip packages), memory & execution timeout guards, LRU warm plugin pool (6 plugins, 3-minute idle TTL), and parallel streaming search with cancellation.
- **Built-in Plugin SDK:** `vessel.http` (with DNS-resolved SSRF protection and per-domain user permission prompts), `vessel.html` (`scraper` CSS selectors), `vessel.crypto` (AES/HMAC/hashes), `vessel.storage`, and `vessel.unpack` (Dean Edwards / custom extractors).
- **Local HTTP / HLS Proxy (`vessel-proxy`):** Binds `127.0.0.1:0` using `axum` + `reqwest` (`rustls`), ephemeral stream tokens, `HMAC-SHA256` signed sub-URLs (open-relay protected), HTTP `206` Range pass-through, `.m3u8` playlist rewriting (`#EXT-X-KEY`, `#EXT-X-MAP`, `#EXT-X-MEDIA`), and Smart Proxy header injection.
- **Metadata Enrichment (`vessel-metadata`):** Zero-key AniList & Cinemeta (IMDb) enrichment plus optional TMDB API key support, `Fix Match` manual canonical ID overrides, and `(season, episode) ↔ absolute_episode` coordinate translation.
- **Persistence & Backups (`vessel-storage`):** `rusqlite` (`bundled` + `WAL` + `FTS5`) single-writer actor managing library items, watch progress, settings, domain grants, metadata cache, IPTV channels, and `.backup.vessel` JSON export/import (`Merge` & `Overwrite` modes).
- **IPTV & EPG (`vessel-iptv`):** Xtream Codes & M3U+ playlist ingestion, XMLTV EPG parsing (Now/Next), and instant SQLite `FTS5` channel search.
- **Offline Downloader (`vessel-downloader`):** HTTP Range resume, 4-worker parallel HLS segment downloader with AES-128 decryption, and subtitle bundling.
- **External `mpv` Backend (`vessel-player-mpv`):** JSON IPC controller (`time-pos`, `eof`, `sub-add`, `sub-delay`) for native `mpv` playback when `mpv` is installed.

### Desktop Application (`apps/desktop` — Tauri v2 + Vue 3 / TypeScript)
- **Nuvio-Inspired Cinema Home:** Full-bleed spotlight hero showcase, **Continue Watching** rail with live progress bars, remaining time, one-click resume, and horizontal content shelves.
- **Full-Screen CloudStream-Style Player (`/watch`):** Auto-hiding HUD, screen lock, in-player **Episodes** drawer, live **Sources** switcher (preserving timestamp), **Subtitles & Sync** (`-500ms / +500ms`), playback speed (`0.5x–2x`), video fit modes (`Fit / Fill / Stretch`), keyboard shortcuts, and optional external **MPV** handoff.
- **Rich Details Page:** Cinematic backdrop, metadata tags, **Fix Match** modal (TMDB / IMDb / AniList or disable enrichment), 3 episode view modes (`Grid Cards`, `Detailed List`, `Compact Rows`), and CloudStream-style **Source Resolver** sheet.
- **Full Suite of Views:** Global streaming **Search** (`Cmd/Ctrl+K`), **Library** watchlist, **Live TV** (IPTV + EPG), **Downloads** queue, **Extensions** manager (URL, shortcode, or local Dev Mode folder), and **Settings** (custom `.theme.vessel` themes, UI preferences, TMDB key, proxy policy, and backups).

---

## Workspace Layout

```text
vessel/
├── crates/
│   ├── vessel-domain       # Core models, VesselError, ports, EventSink, UiPreferences
│   ├── vessel-storage      # SQLite (WAL + FTS5) actor, watch progress, backups, secrets
│   ├── vessel-proxy        # Local 127.0.0.1 HTTP/HLS proxy with HMAC-SHA256 signing
│   ├── vessel-plugin-rt    # QuickJS + oxc TS runtime, SDK bridges, SSRF & domain policy
│   ├── vessel-metadata     # AniList / TMDB / Cinemeta enrichment & episode coordinate sync
│   ├── vessel-iptv         # Xtream Codes, M3U+, XMLTV EPG, FTS5 channel search
│   ├── vessel-downloader   # Range resume & parallel AES-128 HLS offline downloader
│   ├── vessel-player-mpv   # mpv JSON IPC playback backend
│   └── vessel-engine       # Pure-Rust VesselEngine facade orchestrating all subsystems
└── apps/
    └── desktop/            # Tauri v2 + Vue 3 / TypeScript desktop application
```

---

## Installation (Prebuilt Desktop Releases)

Prebuilt desktop installers for **macOS** (`.dmg` for Apple Silicon & Intel) and **Windows** (`-setup.exe`) are published on the [GitHub Releases](https://github.com/falsisdev/vessel/releases) page.

### macOS Note ("Vessel is damaged and can't be opened")
Because Vessel is an open-source app distributed without a paid Apple Developer ID notarization certificate, macOS Gatekeeper flags downloaded `.dmg` apps with a quarantine attribute. After dragging **Vessel.app** into **Applications**, run this once in Terminal:

```bash
xattr -cr /Applications/Vessel.app
```

Then launch **Vessel** normally.

### Windows Note (SmartScreen)
If Windows SmartScreen shows *"Windows protected your PC"* on unsigned alpha/beta installers, click **More info → Run anyway**.

---

## Getting Started

### Prerequisites
- **Rust** (`stable` toolchain via `rustup`)
- **Node.js** (v20+) & **pnpm**
- **Tauri v2 system dependencies** (Xcode Command Line Tools on macOS)
- **Optional:** [`mpv`](https://mpv.io/) (`brew install mpv` on macOS) if you want to hand off playback from the built-in player to native external `mpv`.

### Run the Desktop App (Development)
```bash
pnpm install
pnpm --filter @vessel/desktop tauri dev
```

### Run Tests & Checks
```bash
# Run all Rust workspace tests
cargo test --workspace

# Run Clippy with strict warnings
cargo clippy --workspace --all-targets -- -D warnings

# Typecheck and build the Vue frontend
pnpm --filter @vessel/desktop build
```

---

## Plugin Development (Quick Overview)

Vessel supports loading **Dev Mode** plugins directly from a local folder (`manifest.json` + `index.ts`) via **Extensions → Install Source** (paste the local directory path) or packaged `.plugin.vessel` archives and remote repositories.

A minimal plugin exports:
- `getHomeFeed()` — returns categorized `poster` or `landscape` shelves
- `search(query, filters, page)` — returns matching `MediaSummary` items
- `getDetails(nativeId)` — returns `MediaDetail` and `episodes`
- `getStreams(streamId)` — returns playable `StreamCandidate[]` and `SubtitleTrack[]`

---

## Roadmap (Milestone 6+)

- **Milestone 6.1 — Real Plugins & Desktop v1.0:** Production community plugins, Windows build & `OsKeyringSecretStore`, optional embedded `MpvEmbeddedBackend`, and `.dmg` / `.msi` release packaging.
- **Milestone 6.2 — Android & Android TV:** `crates/vessel-ffi` (`UniFFI`) + `apps/android` (`Kotlin Jetpack Compose` + `Compose for TV` with `mpvkit` / `Media3 ExoPlayer`).
- **Milestone 6.3 — Long-Term:** `iOS` (`MPVKit` LGPL), Torrent / Debrid / Magnet support, Trakt & LAN Sync.

---

## License

Vessel is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See [LICENSE](LICENSE) for the full license text.
