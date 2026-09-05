# ADR 0004: Catalog Providers, Embedded Plugins, and Theme System

## Status
Accepted

## Context
1. A media aggregator requires rich metadata even when stream providers are not installed or offline.
2. Certain core providers (TMDB-based Cinemasis and Sanity-based Mangile) should exist out of the box as unremovable but toggleable defaults.
3. Asian series do not fundamentally differ in data structure from standard episodic television series.
4. Users desire rich customization without breaking native UI responsiveness.

## Decisions

### 1. Catalog vs. Stream Provider Separation
- A provider declaring only `CAPABILITY_SEARCH` and `CAPABILITY_METADATA` operates as a **Catalog Provider**.
- **Cinemasis** serves as the default Cinema catalog provider via TMDB, indexing external IDs (IMDb, SIMKL, MAL, AniList, Kitsu).
- **Mangile** serves as the default Manga, Webook, and Webtoon catalog/reader provider utilizing local database access and Sanity IDs.
- Stream resolution is delegated to providers declaring `CAPABILITY_STREAMS`.

### 2. Unification of Series
- Removed `MediaTypeAsianSeries`. Asian dramas, Kdramas, and anime series with episodic structure are represented as `MediaTypeSeries` or `MediaTypeAnime` with appropriate genre/country tags.

### 3. Embedded Plugin Lifecycle
- Embedded plugins have `is_builtin: true`. They cannot be deleted/uninstalled, but users may disable them via configuration.

### 4. Dual-Dimension Theme System
- Theming is split into two independent domains:
  - **Color Tokens**: Semantic color and styling attributes in JSON/YAML.
  - **Icon Packs**: Vector SVG asset collections.
- Themes install directly from GitHub releases like plugins.

## Consequences
- Clean separation between metadata indexing and video stream resolution.
- UI stays populated with high-quality media catalogs even before stream plugins are configured.
- Seamless community theme ecosystem.
