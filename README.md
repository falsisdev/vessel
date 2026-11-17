# Vessel

**Vessel** is a local-first, cross-platform streaming and media center currently being rewritten from the ground up in **Rust**.

> **Project Status:** Early Development (**Milestone 0** — clean repository setup and core domain architecture).
> The previous Go + SwiftUI prototype has been archived on the `archive/v1-go` branch (`v1-go-archive` tag) and `archive/v1-go-ui-polish` (`v1-go-ui-polish-archive` tag).

---

## Core Principles

- **Local-First & Zero Server Cost:** Scraping, library state, watch progress, metadata caching, offline downloads, and the local HTTP/HLS proxy (`127.0.0.1`) all run directly on the user's device. Zero telemetry.
- **Replaceable Shell, Permanent Brain:** Business logic lives inside UI-agnostic Rust crates (`vessel-core`), keeping UI layers thin and interchangeable across platforms.
- **Self-Contained Plugins + Optional Metadata Enrichment:** Community plugins (TypeScript/JavaScript via `rquickjs` + `oxc`) never depend on external IDs (TMDB/IMDb/AniList) to list episodes or resolve streams, while an optional enrichment layer adds rich posters, backdrops, and cross-provider mapping.
- **Multi-Source Subtitles & Built-in IPTV:** Native support for styled `.ass`/`.ssa`/`.srt` tracks, plugin subtitles, independent subtitle providers, local subtitle files, and Rust-native Xtream Codes / M3U + XMLTV EPG.

## Planned Architecture

- **Core Engine (Rust):** Domain models, `rusqlite` (bundled + WAL + FTS5), `axum` + `reqwest` (`rustls`) local proxy, `rquickjs` + `oxc` plugin runtime, metadata enrichment, IPTV parser, and offline HLS/range downloader.
- **Desktop (`macOS` MVP → `Windows`):** `Tauri v2` + `Vue 3` / `TypeScript` + `libmpv` backend.
- **Mobile & TV (`Android` + `Android TV`):** `Kotlin Jetpack Compose` (+ Compose for TV) bound to `vessel-core` via `UniFFI` (`mpvkit` + `Media3 ExoPlayer`), followed by `iOS` (`MPVKit` LGPL).

## Repository Layout

The workspace is being built incrementally milestone by milestone. Check the commit history as crates and apps are introduced.

## License

Vessel is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See [LICENSE](LICENSE) for the full license text.
