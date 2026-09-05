# Vessel Architecture Master Specification

## 1. Vision & Core Tenets

Vessel is a unified, local-first, modular media consumption platform for Video (Movies, Series, Anime, Live, IPTV) and Reading (Manga, Webtoon, Webook, Book).

### Primary Architectural Axioms
1. **The Backend is Primarily Local**: Core runs locally on user devices. No mandatory central cloud servers for catalog scraping, library storage, or playback orchestration.
2. **Provider Agnostic Core**: Core contains zero website scraping or stream host bypasses.
3. **Out-of-Process Plugins**: Plugins run as separate operating system processes communicating over local gRPC.
4. **Resilient Domain Aggregation**: Core coordinates concurrent plugin queries with strict timeouts. Single plugin crashes or timeouts never degrade the overall user experience.
5. **English Primary, Multilingual by Design**: All internal symbols, contracts, and canonical metadata are English; user interfaces support localization.

---

## 2. Domains & Default Embedded Providers

| Domain | Content Types | Default Embedded Provider | Notes |
| :--- | :--- | :--- | :--- |
| **Cinema** | Movies, Series, Anime | **Cinemasis** (TMDB API) | Catalog/Metadata only. Resolves TMDB, IMDb, SIMKL, MAL, AniList, Kitsu IDs. Does not provide streams. |
| **Manga** | Manga (Page-based) | **Mangile** (Local DB / Sanity IDs) | Default catalog & reader source from local Mangile project database. |
| **Webtoon** | Manhwa, Webtoons (Vertical scroll) | **Mangile** (Local DB / Sanity IDs) | Vertical reading model. |
| **Webook** | Web novels, Light novels (Text-based) | **Mangile** (Local DB / Sanity IDs) | Text/chapter-based reader model. |
| **Book** | EPUB, PDF, Digital Books | *None* (Community / Local Files) | Indexed local files or community plugins. |
| **Live** | Live Streams, Sports, Open TV | *None* (Community / Local) | Community provider plugins. |
| **IPTV** | M3U Playlists, EPG data | *None* (Community / Local) | Community provider plugins. |

### Built-in Provider Lifecycle
- Marked with `is_builtin: true`.
- Cannot be uninstalled from disk.
- Can be toggled on or off by the user in settings.

---

## 3. Protocol & Plugin Architecture

- **Contract**: Protocol Buffers (`proto/plugin/v1/plugin.proto`).
- **Transport**: gRPC over local IPC (Loopback or Unix Domain Sockets).
- **Language Freedom**: Plugins can be implemented in Go, Rust, Python, Kotlin, C#, or any gRPC-capable stack.
- **Capabilities**:
  - `CAPABILITY_SEARCH`: Provider can search its catalog.
  - `CAPABILITY_METADATA`: Provider can return details, genres, seasons, and episodes.
  - `CAPABILITY_STREAMS`: Provider can return playable stream URLs and subtitles.

---

## 4. CSS Theme Architecture

The user interface supports custom community theming without recompiling native clients:
1. **Color Themes (`tokens.json`)**: Design tokens mapping semantic UI elements (background, surface, accent, text, border, blur).
2. **Icon Packs (`icons/`)**: Standalone SVG/vector packs (`play.svg`, `search.svg`, `library.svg`, etc.).
3. **Distribution**: Installed directly from GitHub repositories without a central store.
4. **Embedded Default**: Ships with "Vessel Default Dark/Light" and official icon set.

---

## 5. Media Models

In the Cinema domain:
- `MediaTypeMovie`: Standalone feature films.
- `MediaTypeSeries`: Television, web, and Asian series (unified under series semantics).
- `MediaTypeAnime`: Anime productions with Japanese/global metadata mappings.

---

## 6. Git & Workflow Rules
- **Commit Management**: Commits and pushes are performed by the coding agent automatically upon completing and verifying tasks, using Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, etc.).
