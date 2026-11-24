> 🌐 **English** · [Wiki Home](Home)

# Themes, Localization & Privacy

Vessel treats appearance, layout ergonomics, language accessibility, and user privacy as first-class modular features persisted in SQLite (`UiPreferences`).

---

## 1. Built-in & Custom Themes (`.theme.vessel`)

### 5 Built-in Palettes
1. **Catppuccin Mocha** (`catppuccin-mocha`, dark — default)
2. **Catppuccin Latte** (`catppuccin-latte`, light)
3. **OLED Pitch Black** (`oled-black`, pure `#000000` dark)
4. **Nord** (`nord`, dark)
5. **Tokyo Night** (`tokyo-night`, dark)

### Creating & Importing Custom `.theme.vessel` Themes
Drop a `*.theme.vessel` JSON file into `<VesselRoot>/themes/` or click **Import `.theme.vessel`** in **Settings → Appearance & Theme**:

```json
{
  "schemaVersion": 1,
  "id": "midnight-amber",
  "name": "Midnight Amber",
  "author": "Community",
  "isDark": true,
  "colors": {
    "bgBase": "#141218",
    "bgSurface": "#0d0b10",
    "bgElevated": "#24202b",
    "textPrimary": "#f2edf7",
    "textMuted": "#b0a8bd",
    "accent": "#f59e0b",
    "accentSecondary": "#38bdf8",
    "border": "#363042",
    "success": "#34d399",
    "warning": "#fbbf24",
    "danger": "#f87171"
  }
}
```

All 11 semantic color tokens bind automatically to CSS custom properties (`--vessel-*`) across every view.

---

## 2. Modular Shell Layouts & Custom CSS

Under **Settings → Modular UI Preferences**:
- **6 Navigation Layouts (Desktop):** `Expanded Sidebar`, `Pinned Always-Open Sidebar`, `Floating Bottom Dock`, `Collapsed Icon Rail`, `Top Navbar`, `Hidden` (`Settings` stays accessible).
- **Android Navigation Layout:** Switch between adaptive `Standard` bottom/rail navigation (`NavigationSuiteScaffold`) and a pill-style **`Floating Bottom`** bar.
- **Home Showcase Customization (Desktop & Android):** Configure Spotlight Hero banner mode (`Full`, `Compact`, `Hidden`) and customize card aspect ratios (`poster`, `square`, `landscape` variants).
- **3 Interface Densities:** `Compact`, `Comfortable`, `Spacious`.
- **3 Corner Styles:** `Sharp`, `Rounded`, `Pill`.
- **Card & Details Toggles:** Toggle plugin badges, rating chips, spoiler-free episode overviews/thumbnails, NSFW poster blur, Play Click Behavior (`Quick Start Best` vs `Always Ask Source`), and `Custom CSS Injection` for power users (Desktop).
- **Reset All UI Defaults:** Restores defaults without touching your library, downloads, or plugins.

---

## 3. Localization (`12` Built-in Languages)

Under **Settings → Language & Locales**, pick any bundled language displayed with its localized name:

| Code | Language | Localized Display Name (in EN / TR) |
| :--- | :--- | :--- |
| `en` | English | English / İngilizce |
| `tr` | Turkish | Turkish / Türkçe |
| `es` | Spanish | Spanish / İspanyolca |
| `de` | German | German / Almanca |
| `fr` | French | French / Fransızca |
| `it` | Italian | Italian / İtalyanca |
| `ru` | Russian | Russian / Rusça |
| `ja` | Japanese | Japanese / Japonca |
| `pt` | Portuguese | Portuguese / Portekizce |
| `zh` | Chinese | Chinese / Çince |
| `ar` | Arabic | Arabic / Arapça |
| `fa` | Persian | Persian / Farsça |

> **Custom / Override Locales:** Copy `apps/desktop/src/locales/en.json`, translate keys, and drop `<code >.json` inside `<VesselRoot>/locales/`—Vessel discovers custom locale files automatically!

---

## 4. Privacy Controls

- **Always Proxy Media Streams (`alwaysProxy`):** Forces stream delivery through `127.0.0.1` Smart Proxy so custom `Referer`/`User-Agent` headers remain consistent and browser/WebView preflight leaks are avoided.
- **Disable External Metadata Enrichment (`metadataEnabled: false`):** Halts all background queries to AniList, Cinemeta, AniZip, TMDB, TheIntroDB, IntroDB, and AniSkip. Only active plugin endpoints are contacted.
- **Safe Portable Backups:** Stored API secrets and plugin tokens (`SecretStore`) are **omitted by default** when exporting `.backup.vessel` files so you can share backups safely.
