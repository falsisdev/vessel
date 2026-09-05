# 🚢 Vessel

**Vessel** is an umbrella, local-first, modular digital media consumption and reading platform engineered in Go and modern web standards. It brings Cinema, Television, Manga, Webtoons, Light Novels, and Live IPTV into a unified, lightweight, highly customizable experience.

```
                  ┌─────────────────────────────────────────────────────────┐
                  │                 Vessel Embedded Web UI                  │
                  │   Cinema • Manga/Webtoon • Live IPTV • Offline Library  │
                  └────────────────────────────┬────────────────────────────┘
                                               │ HTTP / REST / SSE / WebSockets
                  ┌────────────────────────────▼────────────────────────────┐
                  │                   Vessel Go Core Engine                 │
                  │  • REST Gateway & Embedded Web Server (:8080)           │
                  │  • Universal Video Player (HLS, MKV, MP4, M3U, YouTube) │
                  │  • Built-in BitTorrent P2P Streaming Engine             │
                  │  • Offline Manga & E-Book Download Manager              │
                  │  • Multi-Device LAN Sync & Remote Control               │
                  │  • Theme Engine & 12-Language i18n Localization         │
                  │  • SQLite Storage & Cross-Device Progress Tracking      │
                  └──────┬──────────────────────┬────────────────────┬──────┘
                         │ gRPC                 │ gRPC               │ gRPC
             ┌───────────▼──────────┐ ┌─────────▼────────┐ ┌─────────▼────────┐
             │  Cinemasis Plugin    │ │  Mangile Plugin  │ │   IPTV Plugin    │
             │  (TMDB Cinema & TV)  │ │ (Manga & Novels) │ │ (Universal & TR) │
             └──────────────────────┘ └──────────────────┘ └──────────────────┘
```

---

## ✨ Key Capabilities

### 🎬 Cinema & Series Domain
* **Multi-Provider Architecture**: Powered by TMDB (`Cinemasis`) and community plugins.
* **Unified Video Player**: Custom-built media player supporting **MKV, MP4, HLS (`.m3u8`), M3U, and YouTube embeds**.
* **Audio & Subtitle Track Switching**: On-the-fly dubbing selection, subtitle toggles, speed control (0.5x - 2.0x), and Picture-in-Picture.
* **Cloud Debrid Streaming**: Native integration with Real-Debrid and TorBox for cached cloud torrent streams.

### 📖 Manga, Webtoon & E-Book Domain
* **Mangile-Inspired Modern Reader**: Fullscreen immersion, continuous webtoon vertical scroll, single-page, and dual-page spread modes.
* **Manga & Light Novel Support**: Reads both image-based chapters and rich typographic novels.
* **Offline Reading Mode**: One-click download (📥) of entire manga chapters or novels for offline reading without an internet connection.
* **Reading Progress Sync**: Remembers exact chapter and page position across sessions.

### 📡 Live IPTV Television
* **Universal World Channels First**: BBC News, Bloomberg TV, Sky News, France 24, Deutsche Welle, CBS News, ABC News, NASA TV, Euronews, TRT World, NHK World.
* **High-Availability Fallbacks & Country Filtering**: Dedicated filters for `ALL`, `TR`, `AZ`, `US`, `UK`, `DE`, `FR`.
* **Zero Broken Logos**: High-resolution Wikimedia verified SVGs and dynamic vector monogram fallbacks.
* **Instant Side-by-Side Playback**: Click any channel to immediately start playback in the integrated player.

### 🧲 BitTorrent P2P Swarm Engine
* **Stream Torrents Directly**: Play magnet links and torrent streams on the fly with zero waiting for complete downloads.
* **Live Swarm Buffer HUD**: Real-time telemetry in the player HUD displaying active peers, seeders, buffer percentage, and download speed (MB/s).
* **Sequential HTTP 206 Partial Content**: Enables instant scrubbing and seeking over P2P swarms.

### 📱 Multi-Device LAN Sync & Remote Control
* **Zero-Config LAN Discovery**: Automatically discovers active Vessel instances on your local Wi-Fi or LAN.
* **Remote Control Pad**: Control playback (Play, Pause, Seek, Volume) on another computer, TV, or tablet across the room.
* **Instant Stream Casting**: Cast active streams or chapters to any discovered LAN node with a single click.

### 🔌 Community Plugin Ecosystem & SDK
* **Decoupled Out-of-Process Architecture**: Plugins run independently communicating via standard gRPC and Protocol Buffers.
* **Developer SDK**: Built-in CLI commands to scaffold and validate community plugins:
  ```bash
  vessel plugin init ./my-provider    # Scaffolds ready-to-run Go gRPC plugin
  vessel plugin validate ./my-provider # Validates manifest, executable, and schema
  ```
* **Plugin Priority Ordering**: Reorder catalog and search priority with instant drag/reorder controls.
* For full plugin development instructions, see [Plugin SDK Documentation](docs/plugin-sdk.md).

### 🌐 12-Locale Internationalization & Curated Themes
* **Default Language**: English (`en`), with full support for Turkish (`tr`), Azerbaijani (`az`), German (`de`), French (`fr`), Spanish (`es`), Portuguese (`pt`), Russian (`ru`), Japanese (`ja`), Chinese (`zh`), Arabic (`ar` RTL), and Persian (`fa` RTL).
* **Reading-First Color Themes**: Curated palettes including *Mangile Duman* (misty slate default), *Mangile Leylak* (mauve), *Mangile Kaya* (stone), *Midnight OLED*, *Catppuccin*, *Nord*, and *Dracula*.

---

## 🚀 Quick Start

Run Vessel on **macOS, Linux, Windows, Raspberry Pi, home servers, or NAS** without requiring any desktop GUI packages.

### 1. Launch Vessel Instantly

```bash
# Using the startup script:
./scripts/start.sh

# Or directly with Go:
go run ./core/cmd/vessel serve -open
```

Open **`http://127.0.0.1:8080`** in your browser.

### 2. Access from Phone, Tablet, or LAN Devices

To broadcast across your local network:

```bash
./bin/vessel serve -host 0.0.0.0 -port 8080
```

Vessel will display reachable LAN URLs (e.g. `http://192.168.1.45:8080`).

---

## 💻 CLI Commands

Vessel includes a comprehensive CLI for server management, themes, and plugins:

```bash
# --- Server Commands ---
./bin/vessel serve                       # Start background daemon and HTTP gateway
./bin/vessel serve -port 9000            # Custom port
./bin/vessel serve -host 0.0.0.0         # Listen on all interfaces

# --- Plugin Management ---
./bin/vessel plugin list                 # List discovered & connected plugins
./bin/vessel plugin enable <id>          # Enable an installed plugin
./bin/vessel plugin disable <id>         # Disable a plugin
./bin/vessel plugin install <path|url>   # Install a plugin from path or URL
./bin/vessel plugin init <dir>           # Scaffold a new community plugin template
./bin/vessel plugin validate <dir>       # Validate plugin manifest and binary

# --- Theme Management ---
./bin/vessel theme list                  # List all available built-in & custom themes
./bin/vessel theme apply mangile         # Apply Mangile Duman dark slate
./bin/vessel theme apply midnight-oled   # Apply true-black OLED theme
./bin/vessel theme install <path|url>    # Install community CSS theme (.zip or URL)
```

---

## 📁 Project Architecture

```
vessel/
├── core/
│   ├── cmd/vessel/             # Unified CLI & server entrypoint
│   └── internal/
│       ├── cli/                # Cobra CLI commands (serve, theme, plugin init/validate)
│       ├── debrid/             # Real-Debrid & TorBox streaming clients
│       ├── domain/             # Domain entities (cinema, reading, library, locale)
│       ├── plugin/             # gRPC plugin supervisor & process lifecycle manager
│       ├── server/             # REST HTTP Gateway & Range-enabled proxy
│       ├── service/            # Core business logic & DownloadService
│       ├── storage/            # SQLite persistent store & resume progress
│       ├── streaming/          # BitTorrent P2P streaming engine
│       ├── sync/               # UDP LAN beacon discovery & remote command sync
│       └── theme/              # CSS theme compiler & palette loader
├── plugins/
│   ├── cinemasis/              # Universal TMDB cinema & series catalog provider
│   ├── mangile/                # Sanity-powered Manga, Webtoon & Webook provider
│   ├── iptv/                   # Global IPTV live channels provider
│   └── examples/cinema-mock/   # Reference standalone plugin implementation
├── ui/
│   ├── index.html              # Modern responsive single-page application
│   ├── app.js                  # Reactive UI controller (Player, Reader, LAN, i18n)
│   ├── styles.css              # Design tokens & responsive styles
│   └── embed.go                # Go standard embed packaging
├── docs/
│   └── plugin-sdk.md           # Plugin SDK specifications & developer guide
├── proto/                      # Protobuf gRPC contracts (Core & Plugin v1)
└── scripts/                    # Test, build, and bootstrap utilities
```

---

## 🛠️ Testing & Building

### Run Full Test Suite

```bash
./scripts/test.sh
# or directly:
go test ./...
```

### Compile Production Binary

```bash
go build -o ./bin/vessel ./core/cmd/vessel
```

---

## 📄 License

Vessel is released under the [MIT License](LICENSE).

