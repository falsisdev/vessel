# Vessel Architecture Overview

Vessel is a local-first, modular, cross-platform digital media consumption platform.

## High-Level Topology

```
┌─────────────────────────────────────────────────┐
│               Native UI Clients                 │
│   (macOS SwiftUI, iOS, Android Compose, etc.)   │
└────────────────────────┬────────────────────────┘
                         │ CoreClient (IPC / gRPC)
┌────────────────────────▼────────────────────────┐
│                   Vessel Core                   │
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │             Domain Services               │  │
│  │   Cinema, Manga, Webtoon, Book, Live...   │  │
│  └─────────────────────┬─────────────────────┘  │
│                        │                        │
│  ┌─────────────────────▼─────────────────────┐  │
│  │              Plugin Manager               │  │
│  │   Lifecycle, Health, RPC Client Registry  │  │
│  └─────────────────────┬─────────────────────┘  │
└────────────────────────┼────────────────────────┘
                         │ gRPC (IPC / Loopback)
┌────────────────────────▼────────────────────────┐
│               External Plugins                  │
│   Cinema Mock, Anime, Manga, IPTV, etc.        │
└─────────────────────────────────────────────────┘
```

## Core Principles

1. **Local-First Runtime**: Core runs on the user device. There is no centralized backend handling scraping or streaming.
2. **Strict Provider Isolation**: Core never parses HTML or bypasses anti-bot mechanisms. All provider interaction lives inside plugins.
3. **Out-of-Process Plugins**: Plugins run as separate processes communicating over local gRPC, ensuring crash isolation and multi-language support.
4. **Protobuf Contracts**: Single source of truth defined in `proto/plugin/v1/plugin.proto`.
5. **Resilient Aggregation**: Slow or crashing plugins are cancelled or skipped without failing user requests.
