> 🌐 **Languages:** **English** · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Welcome to the Vessel Wiki

**Vessel** is a **local-first, privacy-respecting, cross-platform streaming and media center** written in **Rust**, powered by **Tauri v2** and **Vue 3 / TypeScript**.

> *"Most ships dread Rust. This cargo vessel runs on it — shipped with Cargo, engineered not to rust, docking plugins by manifest at local ports."*

Unlike cloud-reliant streaming hubs or closed-source scrapers, Vessel executes **every component locally on your hardware**: TypeScript/JavaScript extensions inside isolated QuickJS sandboxes, an `HMAC-SHA256` authenticated HTTP/HLS loopback proxy (`127.0.0.1`), single-writer SQLite (`WAL` + `FTS5`) persistence, concurrent offline HLS/Range downloads, IPTV + XMLTV EPG indexing, multi-provider skip segments (`Intro`, `Recap`, `Credits`, `Preview`), and native `mpv` handoff.

---

## Core Architectural Principles

1. **Local-First & Zero Server Cost:** Zero central servers, zero accounts, zero telemetry. Everything runs offline-capable on localhost.
2. **Replaceable Shell, Permanent Brain:** 100% of core logic sits inside UI-agnostic Rust crates (`vessel-engine` + workspace crates). Desktop (`Tauri v2`) and future `UniFFI` Android / Android TV shells stay thin.
3. **Self-Contained Plugins + Optional Metadata Enrichment:** Every plugin can browse, search, list episodes, and play streams independently—even when external metadata APIs are disabled in Settings.
4. **Domain-Level Security by Default:** Extensions are sandboxed (`32 MiB` heap limit, `512 KiB` stack limit, `15s` execution watchdog), have no filesystem read access, pass DNS-resolved SSRF checks, and prompt the user (`Allow Once` / `Always Allow` / `Reject`) before contacting unapproved domains.

---

## Quick Navigation

| Guide | What You Will Learn |
| :--- | :--- |
| **[Installation & Setup](Installation-and-Setup)** | Prebuilt macOS (`.dmg`) & Windows (`.exe`) installers, Gatekeeper `xattr -cr` fix, `<VesselRoot>` directory tree, Auto-Updater, Building from Source |
| **[Desktop App User Guide](Desktop-App-User-Guide)** | Spotlight Hero, **Smart Continue Watching** & Manual Editor, Search filters, Details layouts & **Fix Match**, Quick Watch Statuses, `.backup.vessel` backups |
| **[Player & Skip Segments](Player-Controls-and-Skip-Segments)** | CloudStream-style `/watch` HUD, Audio track switcher, Subtitle styling & sync, Keyboard shortcuts, Touchpad gestures, **Skip Intro/Recap/Credits/Preview** |
| **[Offline Downloads & Queue](Offline-Downloads-and-Queue)** | 8-worker AES-128 HLS & Range multipart downloads, audio+subtitle bundling, instant Pause/Delete, Drag-and-Drop Queue, **4-Level Hierarchy** |
| **[Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG)** | M3U+ & Xtream Codes, XMLTV Now/Next EPG, SQLite FTS5 search, Favorites, Custom User-Agent, Live TV Plugins |
| **[Metadata & Episode Sync](Metadata-and-Episode-Sync)** | AniList, Cinemeta, AniZip, optional TMDB, Privacy Opt-Out, Fix Match, `(season, episode) ↔ absolute_episode` alignment |
| **[Extensions & Repositories](Extensions-and-Repositories)** | `gh:`, `gl:`, shortcodes, `vessel://install`, Dev Mode, Trust Badges, Nuvio scraper compatibility, Domain permissions |
| **[Plugin Development SDK](Plugin-Development-SDK)** | Writing TypeScript plugins, `manifest.json`, `repo.json`, `vessel.*` APIs (`http`, `html`, `crypto`, `storage`, `unpack`), Nuvio shims |
| **[Architecture & Crates](Architecture-and-Crates)** | All 10 Rust workspace crates, Mermaid dataflow diagram, SQLite actor, Proxy HMAC signing, `[Vessel<Variant>Error]` taxonomy |
| **[Themes, i18n & Privacy](Themes-Localization-and-Privacy)** | 5 built-in themes, `.theme.vessel` tokens, Modular Nav layouts, Custom CSS, 12 Languages, Privacy switches |

---

## Feature Matrix (`v0.1.44`)

| Area | Implemented Capabilities |
| :--- | :--- |
| **Plugin Runtime** | `rquickjs` + `oxc` TypeScript stripper, SQLite bytecode/JS cache, ES6 modules, 18-slot LRU warm pool (5m TTL), Nuvio shims (`cheerio`, `axios`, `CryptoJS`, `fetch`) |
| **Streaming & Proxy** | Local `127.0.0.1` `axum` server, `HMAC-SHA256` sub-URLs, Smart Proxy header injection, `.m3u8` AES-128 key/segment rewriting, `206 Partial Content` local offline server |
| **Playback (`/watch`)** | In-app HLS (`hls.js`) + MPEG-TS (`mpegts.js`) + native WebKit + external `mpv` JSON IPC, multi-track Audio, Subtitles (`srt/vtt/ass/ssa`) + sync, Gestures, Skip button |
| **Skip Segments** | Priority cascade **`Plugin > TheIntroDB > IntroDB > AniSkip`** (`AniZip` ID resolution, AniSkip season/duration sanity guards, IntroDB confidence/negative flags) |
| **Continue Watching** | Smart series auto-advance on completion, hides completed movies/finales, right-click/long-press *Go to Details*, Manual Progress Editor, per-episode bars |
| **Offline Library** | 4-level **`Content → Season → Episode → Streams`** tree (`Content → Streams` for movies), parallel audio/subtitle downloads, instant cancel, draggable Queue |
| **IPTV** | Xtream Codes + M3U+, XMLTV EPG, FTS5 search, Favorites, dedicated **Live TV Plugins** tab |
| **Customization** | 5 built-in themes + `.theme.vessel` JSON, 6 nav layouts (incl. `Floating Bottom Dock`), 3 densities, 12 languages + custom `<VesselRoot>/locales`, Custom CSS |

---

## Roadmap (Milestone 6+)

- **Milestone 6.1 — Production Ecosystem & Desktop v1.0:** Real-world community plugins, Windows `OsKeyringSecretStore`, optional embedded `MpvEmbeddedBackend`, stable signed `.dmg` / `.msi` / `.exe` releases.
- **Milestone 6.2 — Android & Android TV (`UniFFI`):** Shared Rust `vessel-engine` via `crates/vessel-ffi` powering Kotlin Jetpack Compose + Compose for TV (`mpvkit` / `Media3 ExoPlayer`).
- **Milestone 6.1–6.2 (Current `v0.1.44`):** Cross-platform **Desktop (Tauri v2 + Vue 3)** & **Android (`apps/android` Kotlin Jetpack Compose + Media3 ExoPlayer via `vessel-ffi`)**, universal third-party Nuvio/Stremio/ESM/TS runtime compatibility, 6-provider Fix Match title search, cross-source stream linking, Live TV customization & favorites, and true edge-to-edge Android Floating Bottom Navbar.
- **Milestone 6.3+ — Roadmap:**
  1. Unified OAuth (Loopback / Deep Link / TV Device Code) + `OsKeyringSecretStore` token security.
  2. SQLite deletion tombstones + Google Drive sync ↔ **Simkl / Trakt / AniList / MAL / Kitsu** library/scrobble sync + external subtitle providers (**OpenSubtitles / SubDL**).
  3. Memory/cache optimizations, **Android TV (`Compose for TV`)**, and optional Android `mpvkit` (`libass`) / Desktop embedded `libmpv` HUD.
  4. **iOS & iPadOS** (`SwiftUI` + `vessel-ffi` + `MPVKit`), Torrent/Debrid/Magnet, and optional Desktop `ffmpeg -c copy` `.mp4` export.
