# Vessel

Vessel is an umbrella, local-first, modular digital media consumption and reading platform.

---

## 🌐 Quick Start: Web UI (Run Anywhere Without Desktop App)

You can run Vessel on **any machine** (macOS, Linux, Windows, Raspberry Pi, home server, NAS) without installing a desktop `.app` or GUI package. All frontend assets and backend services are compiled into a single executable and accessible from any web browser.

### 1. Start Vessel Instantly

```bash
# Using the quick startup script:
./scripts/start.sh

# Or directly with Go:
go run ./core/cmd/vessel serve -open
```

Once started, open **`http://127.0.0.1:8080`** in your browser.

### 2. Access from Phone, Tablet, or LAN Devices

To access Vessel from your mobile phone, iPad, or another computer on the same Wi-Fi network:

```bash
./bin/vessel serve -host 0.0.0.0 -port 8080
```

Vessel will display your local IP addresses in the terminal (e.g. `http://192.168.1.45:8080`).

---

## 🎨 Themes & CLI Management

Vessel features a built-in theme engine with curated color palettes, including authentic manga & novel reading themes:

* **Mangile Duman** (`mangile`): Misty dark slate with clean white accents (Default)
* **Mangile Leylak** (`mangile-mauve`): Elegant subtle mauve dark with clean white accents
* **Mangile Kaya** (`mangile-stone`): Warm earthy dark stone tones
* **Mangile Çinko** (`mangile-zinc`): Deep modern zinc neutral
* **Mangile Arduvaz** (`mangile-slate`): Deep navy slate
* **Mangile Zeytin** (`mangile-olive`): Forest olive dark
* **Mangile Boz** (`mangile-taupe`): Warm taupe gray
* **Mangile Kır** (`mangile-gray`): Cool gray dark
* **Mangile Yavan** (`mangile-neutral`): True monochrome dark
* **Vessel Dark / Light**, **Midnight OLED**, **Catppuccin**, **Nord**, **Dracula**

### Theme Management via CLI

```bash
# List all installed and built-in themes
./bin/vessel theme list

# Switch active theme instantly
./bin/vessel theme apply mangile-mauve
./bin/vessel theme apply mangile-stone

# Install community theme (.zip, folder, or URL)
./bin/vessel theme install https://example.com/themes/cyberpunk.zip
```

### Plugin Management via CLI

```bash
# List connected and discovered plugins
./bin/vessel plugin list

# Install a plugin package (.zip, folder, or URL)
./bin/vessel plugin install ./path/to/plugin

# Enable or disable plugins
./bin/vessel plugin disable com.test.anime
./bin/vessel plugin enable com.test.anime
```

---

## 📁 Repository Layout

```
.
├── core/                       # Go Core application runtime & CLI
│   ├── cmd/vessel/             # Unified Vessel binary entrypoint
│   ├── cmd/core/               # Daemon compatibility entrypoint
│   ├── internal/
│   │   ├── cli/                # Plugin, theme, and server CLI engine
│   │   ├── client/             # Go Core IPC client SDK
│   │   ├── debrid/             # Real-Debrid and TorBox streaming engines
│   │   ├── domain/             # Cinema, reading, library, locale models
│   │   ├── plugin/             # Plugin gRPC client and supervisor manager
│   │   ├── server/             # Core IPC gRPC server & REST/Web Gateway
│   │   ├── service/            # Cinema, reading, library, and stream services
│   │   ├── storage/            # SQLite storage & progress tracking
│   │   ├── streaming/          # Range-enabled streaming proxy
│   │   └── theme/              # Curated theme engine & token compiler
│   └── test/integration/       # End-to-end integration tests
├── plugins/
│   ├── cinemasis/              # Official built-in TMDB Cinema catalog plugin
│   ├── mangile/                # Official built-in Sanity Manga & Novel plugin
│   └── examples/cinema-mock/   # Reference standalone Cinema plugin
├── ui/                         # Embedded modern responsive Web UI
│   ├── embed.go                # Go standard embed filesystem
│   ├── index.html              # Single-page application shell
│   ├── app.js                  # Reactive UI controller with 12-locale i18n
│   └── styles.css              # Design tokens and theme styling
├── proto/                      # Protocol Buffers definitions
│   ├── core/v1/core.proto      # Core IPC contract
│   ├── plugin/v1/plugin.proto  # Plugin contract
│   └── gen/go/                 # Generated Go gRPC/Protobuf bindings
└── scripts/                    # Build, test, and startup scripts
    ├── start.sh                # Zero-config launch script
    ├── test.sh                 # Test suite runner
    └── buf-generate.sh         # Protobuf code generator
```

---

## 🛠️ Development & Testing

### Prerequisites
- [Go](https://go.dev/) (1.22+)
- [Buf](https://buf.build/) (for Protobuf workflows)
- [Protoc](https://github.com/protocolbuffers/protobuf)

### Run Tests
```bash
./scripts/test.sh
```

### Build CLI Binary
```bash
go build -o ./bin/vessel ./core/cmd/vessel
```
