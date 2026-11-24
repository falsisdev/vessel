> 🌐 **English** · [Türkçe](Installation-and-Setup-tr) · [Wiki Home](Home)

# Installation & Setup

This guide covers installing prebuilt **Vessel** Desktop (`macOS`, `Windows`, `Linux`) and **Android** (`.apk`) releases, locating your local data directory (`<VesselRoot>`), configuring updates, and building Vessel from source.

---

## 1. Prebuilt Desktop & Android Releases

Official Desktop installers are built automatically via GitHub Actions on every tagged release (`v*`) and published alongside Android APK builds on **[GitHub Releases](https://github.com/falsisdev/vessel/releases)**:

| Platform | Architecture | Package Format | Notes |
| :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon (`aarch64-apple-darwin`) | `.dmg` | Drag `Vessel.app` to Applications |
| **macOS** | Intel (`x86_64-apple-darwin`) | `.dmg` | Drag `Vessel.app` to Applications |
| **Windows** | x64 (`x86_64-pc-windows-msvc`) | `-setup.exe` (NSIS) | Built-in WebView2 runtime check |
| **Linux** | x64 (`x86_64-unknown-linux-gnu`) | `.AppImage`, `.deb`, `.rpm`, `.flatpak` | Sandbox-aware Flatpak (`dev.falsis.vessel`) + native packages |
| **Android** | ARM64 / ARMv7 / x86_64 (`Universal`) | `.apk` | Direct APK install (`Vessel_*_android.apk`) + in-app update checker |

---

### Linux Installation (`.AppImage`, `.deb`, `.rpm`, `.flatpak`)

- **AppImage:** `chmod +x Vessel_*.AppImage && ./Vessel_*.AppImage` (bundles GStreamer media framework libraries).
- **Debian / Ubuntu (`.deb`):** `sudo apt install ./Vessel_*.deb`
- **Fedora / RHEL (`.rpm`):** `sudo dnf install ./Vessel-*.rpm`
- **Flatpak (`.flatpak` bundle):**
  ```bash
  flatpak install --user Vessel_*_amd64.flatpak
  flatpak run dev.falsis.vessel
  ```
  > Under Flatpak, `<VesselRoot>` defaults to Tauri's XDG data directory (`~/.var/app/dev.falsis.vessel/data/dev.falsis.vessel`), while custom download locations can still be selected via portal/XDG permissions.

---

### macOS Gatekeeper Note (`"Vessel is damaged and can't be opened"`)

Because Vessel is an open-source project distributed without a paid Apple Developer ID notarization certificate, macOS Gatekeeper attaches a `com.apple.quarantine` attribute to binaries downloaded via a web browser.

1. Open the downloaded `.dmg` and drag **Vessel.app** into your **Applications** folder.
2. Open **Terminal** and run once:
   ```bash
   xattr -cr /Applications/Vessel.app
   ```
3. Launch **Vessel** normally from Applications or Spotlight.

> If you install a new update `.dmg` later, you may need to re-run `xattr -cr /Applications/Vessel.app` once if macOS flags the replaced bundle.

---

### Windows SmartScreen Note

On unsigned alpha/beta Windows installers (`*-setup.exe`), Windows Defender SmartScreen may display *"Windows protected your PC"*:
1. Click **More info**.
2. Click **Run anyway**.

---

### Optional: External `mpv` Backend

Vessel plays HLS, MP4, WebM, and MPEG-TS streams directly inside its WebView player (`hls.js` + `mpegts.js`). However, formats such as **MKV**, **HEVC/H.265**, or exotic multi-track audio codecs on Windows/Linux WebViews can be handed off with one click to native **`mpv`** when installed in your `$PATH`:

- **macOS (Homebrew):** `brew install mpv`
- **Windows (winget / Scoop):** `winget install mpv.net` or `scoop install mpv`
- **Linux (apt / pacman):** `sudo apt install mpv` / `sudo pacman -S mpv`

---

## 2. Local Data Directory (`<VesselRoot>`)

Vessel never syncs your data to any cloud server. Everything resides locally inside `<VesselRoot>` (configurable or inspectable from **Settings**):

```text
<VesselRoot>/
├── storage.db           # SQLite (WAL + FTS5): library, watch progress, plugins, domain grants, IPTV, metadata cache
├── downloads/           # Default offline download folder (can be changed in Settings -> Download Location)
│   └── <download_id>/   # Downloaded media, master.m3u8, local segments, bundled audio tracks & subtitles
├── plugins/             # Installed .plugin.vessel packages and extracted runtime files
├── themes/              # Imported custom *.theme.vessel JSON color palettes
├── locales/             # Drop custom <lang>.json UI translations here (loaded alongside the 12 built-in languages)
└── backups/             # Default directory for exported *.backup.vessel snapshots
```

---

## 3. Built-in Auto-Updater & Patch Notes

Inside **Settings → Updates & Release Notes**:
- Vessel checks the GitHub Releases API (`falsisdev/vessel/releases`) for newer tags (supporting alpha/beta pre-release fallback while in pre-1.0).
- When a newer version is available, Vessel displays the **Patch Notes** modal directly inside the app and can fetch installer assets strictly restricted to verified GitHub release hosts.

---

## 4. Building from Source

### Prerequisites
1. **Rust Toolchain:** `1.89+` (`rustup default stable`)
2. **Node.js & pnpm:** Node.js `v20+` (CI uses `v22`) and `pnpm` (`v10+`, e.g. `corepack enable`)
3. **Tauri v2 System Dependencies:**
   - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
   - **Windows:** Microsoft C++ Build Tools + WebView2 Runtime
   - **Linux:** `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Development Commands
```bash
# Clone the repository
git clone https://github.com/falsisdev/vessel.git
cd vessel

# Install frontend workspace dependencies
pnpm install

# Launch desktop app in hot-reload dev mode (Vite on port 1420 + Rust Tauri shell)
pnpm --filter @vessel/desktop tauri dev
```

### Verification & Production Bundling
```bash
# Check Rust formatting
cargo fmt --all -- --check

# Run Clippy with all warnings treated as errors
cargo clippy --workspace --all-targets -- -D warnings

# Run all Rust unit & integration tests across all 10 workspace crates
cargo test --workspace

# Typecheck Vue 3 + TypeScript and build frontend bundle
pnpm --filter @vessel/desktop build

# Build production installer bundle (.dmg on macOS / -setup.exe on Windows)
pnpm --filter @vessel/desktop tauri build
```
