# Vessel Plugin SDK & Developer Guide

Welcome to the **Vessel Plugin Development Kit**. Vessel is engineered from the ground up to be modular, extensible, and language-agnostic. Anyone can build custom catalog providers, scrapers, debrid resolvers, reading providers, or IPTV integrations in **any language** (Go, Rust, Python, Node.js, C#, etc.).

---

## 1. Architectural Overview

Vessel utilizes an **out-of-process architecture** communicating over **gRPC (Protocol Buffers)** through local loopback TCP sockets.

```
┌─────────────────────────────────────────────────────────┐
│                      Vessel Core                        │
│   (Domain Services, Library DB, HTTP/REST API, Web UI)  │
└────────────────────────────┬────────────────────────────┘
                             │ gRPC over 127.0.0.1 (Ephemeral Port)
              ┌──────────────┴──────────────┐
              ▼                             ▼
┌───────────────────────────┐ ┌───────────────────────────┐
│     Cinemasis Plugin      │ │      Community Plugin     │
│   (TMDB / Cinema / HLS)   │ │  (Go, Python, Rust, Node) │
└───────────────────────────┘ └───────────────────────────┘
```

### Key Principles:
1. **Crash Isolation**: A failure or crash in a community plugin does not affect Vessel Core or other plugins.
2. **Language Agnostic**: Plugins compile into standalone executables or scripts that communicate over standard gRPC.
3. **Zero Configuration Discovery**: The executable outputs `VESSEL_PLUGIN_PORT=<port>` to `stdout` upon startup; Vessel Core connects immediately.

---

## 2. Plugin Manifest (`plugin.json`)

Every plugin must contain a `plugin.json` manifest at its root:

```json
{
  "id": "com.example.myprovider",
  "name": "My Media Provider",
  "version": "1.0.0",
  "description": "Streaming catalog and search provider for community movies and shows",
  "author": "Your Name <you@example.com>",
  "domain": "cinema",
  "protocol_version": "1.0.0",
  "executable": "./bin/myprovider",
  "capabilities": [
    "search",
    "metadata",
    "streams"
  ],
  "languages": [
    "en",
    "tr",
    "multilingual"
  ],
  "language_display": "🌐 Çok Dilli / Multilingual"
}
```

### Field Definitions

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | **Yes** | Unique reverse-DNS identifier (e.g. `com.developer.provider`). |
| `name` | `string` | **Yes** | Human-readable title displayed in Vessel UI and catalogs. |
| `version` | `string` | **Yes** | Semantic version (e.g. `1.0.0`). |
| `description` | `string` | No | Short description of media provided by this plugin. |
| `author` | `string` | No | Name or email of plugin author/maintainer. |
| `domain` | `string` | **Yes** | One of: `cinema`, `reading`, `manga`, `iptv`, `live`. |
| `protocol_version`| `string` | **Yes** | Vessel IPC Protocol version (currently `1.0.0`). |
| `executable` | `string` | **Yes** | Relative path to the compiled binary or startup script. |
| `capabilities` | `string[]` | **Yes** | Array containing one or more of: `search`, `metadata`, `streams`, `catalog`. |
| `languages` | `string[]` | No | ISO 639-1 language codes or `multilingual`. |
| `language_display`| `string` | No | Display badge label in UI (e.g. `🇹🇷 TR`, `🌐 Universal`). |

---

## 3. Quick Start: Scaffolding with the Vessel CLI

You can generate a fully functioning plugin boilerplate in seconds using the Vessel CLI:

```bash
# Scaffold a new cinema plugin
vessel plugin init ./plugins/my-cinema --id com.community.cinema --name "Community Cinema" --domain cinema

# Scaffold a new manga/reading plugin
vessel plugin init ./plugins/my-manga --id com.community.manga --name "Community Manga" --domain reading
```

### Scaffolded Files
- `plugin.json`: Metadata manifest.
- `main.go`: Fully functional gRPC server with sample items.
- `README.md`: Compilation and testing commands.

---

## 4. Validating and Testing Plugins

Before distributing your plugin, test its structure and binary:

```bash
# Validate plugin manifest and executable permissions
vessel plugin validate ./plugins/my-cinema

# Output:
# 🔍 Validating Vessel Plugin: ./plugins/my-cinema/plugin.json
#   [✓] Manifest JSON syntax valid
#   [✓] Required metadata present: ID=com.community.cinema, Name=Community Cinema, Domain=cinema, Version=1.0.0
#   [✓] Domain 'cinema' recognized
#   [✓] Executable verified: ./plugins/my-cinema/my-cinema
# 🎉 Plugin validation passed successfully!
```

---

## 5. Installing and Testing in Vessel

Install your local plugin directly into your Vessel environment:

```bash
# Install from local directory
vessel plugin install ./plugins/my-cinema

# Verify installation
vessel plugin list

# Run Vessel runtime
vessel serve
```

Your plugin's catalogs will now appear on the Vessel Home screen, and its media will be indexed in full-page multi-domain search!

---

## 6. Distributing Community Plugins

Plugins can be shared with other users via:
1. **Direct URL or Archive**: Host a `.zip` or `.tar.gz` package containing `plugin.json` and the compiled binary. Users can install with:
   ```bash
   vessel plugin install https://example.com/plugins/my-cinema.tar.gz
   ```
2. **Git Repository**:
   ```bash
   git clone https://github.com/user/vessel-plugin-sample
   vessel plugin install ./vessel-plugin-sample
   ```
