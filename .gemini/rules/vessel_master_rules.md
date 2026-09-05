# Vessel - Project Master Rules & Constitution

These rules are permanent and MUST be followed in every conversation, task, and iteration for this project.

## 1. Core Communication & Coding Rules
- **Language with User**: Always speak Turkish with the user.
- **Code Comments**: Do NOT add redundant, verbose, or AI-generated comments. Code comments must be minimal, concise, and natural, exactly as if written by the developer himself.
- **Primary Project Language**: The application is multilingual, but English is the primary language for all code, protobuf definitions, logs, documentation, architecture records, and user-facing assets.

## 2. Core Architectural Principles
- **Local-First Runtime**: The Core is a local Go application runtime on the user's device. No mandatory remote backend.
- **Process Isolation**: Core and Plugins run as separate processes communicating over local gRPC. Plugins can be written in any language (Go, Rust, Python, etc.).
- **Strict Provider Isolation**: Zero scraping, zero HTML parsing, and zero anti-bot/Cloudflare logic in Core. All provider-specific logic strictly lives inside plugins.
- **Resilient Aggregation**: Domain services (e.g. Cinema, Manga) query plugins concurrently with deadlines. Slow or failing plugins do not break the aggregated response.

## 3. Embedded / Built-in Plugins
- **Cinema Domain -> Cinemasis**:
  - Acts as a pure **Catalog / Metadata Provider** using TMDB API.
  - Maps entries to external IDs: IMDb, SIMKL, and where available MyAnimeList (MAL), AniList, and Kitsu.
  - Does NOT provide video streams. It provides rich metadata regardless of whether stream providers exist.
  - If a community stream plugin is present, Core resolves streams using the identifiers provided by Cinemasis.
- **Manga, Webook (Web novels/light novels), Webtoon Domains -> Mangile**:
  - Connects to the local Mangile project database available on the user's device.
  - Serves data normalized with Sanity IDs.
- **Live, IPTV, Book Domains**:
  - No default embedded plugins provided out of the box. Supported via local files or community plugins.
- **Lifecycle Constraint**:
  - Embedded plugins are flagged with `is_builtin: true`.
  - They CANNOT be uninstalled by the user, but CAN be toggled on/off (disabled/enabled).

## 4. Community Plugin & Theme Ecosystem
- **No Central Store Required**: Plugins and themes install directly from GitHub repositories and GitHub Releases.
- **Spicetify-like Theme System**:
  - Installed via GitHub similar to plugins.
  - Separated into two independent dimensions:
    1. **Color Themes (Tokens)**: JSON/YAML palette variables (background, surface, accent, text, border, glassmorphism).
    2. **Icon Packs**: SVG/vector asset bundles (`play`, `search`, `library`, `domain icons`, etc.).
  - Ships with an embedded default theme (Vessel Default Dark/Light). Community themes can override color and icons independently.

## 5. Media Types
- In Cinema domain: `Movie`, `Series`, `Anime`. (Asian series are treated as `Series`, not a distinct enum type).
