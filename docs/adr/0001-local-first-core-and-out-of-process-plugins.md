# ADR 0001: Local-First Core and Out-of-Process Plugins

## Status
Accepted

## Context
Traditional media streaming and catalog platforms rely on centralized, remote backends to scrape websites, search providers, manage user libraries, and resolve streams. This creates high operational costs, vendor lock-in, latency, privacy issues, and single points of failure.
Additionally, plugins in community ecosystems frequently crash, consume excessive resources, or present security risks.

## Decision
1. **Local-First Core**: The application runtime ("Core") runs entirely on the user's device (desktop, mobile, local network), written in Go for cross-platform portability, low overhead, and efficient concurrency.
2. **Out-of-Process Plugins**: Plugins run as separate processes communicating with Core over local IPC/gRPC.
3. **Crash Isolation**: A crash or memory leak in a third-party plugin does not crash the Core or native UI.
4. **No Mandatory Cloud Backend**: Core operates independently without a hosted backend. Cloud synchronization remains an optional add-on for user state only.

## Consequences
- Requires local process and lifecycle management (spawning, health monitoring, shutdown).
- IPC overhead is minimal (sub-millisecond over loopback or Unix domain sockets).
- Allows plugins to be implemented in any programming language capable of gRPC.
