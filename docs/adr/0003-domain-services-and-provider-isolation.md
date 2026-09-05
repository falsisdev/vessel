# ADR 0003: Domain Services and Strict Provider Isolation

## Status
Accepted

## Context
In media aggregators, scraping logic and provider-specific quirks (Cloudflare challenges, specific query parameters, bespoke DOM structures) frequently leak into the core application logic. This makes the core fragile, hard to maintain, and prone to legal exposure.

## Decision
1. **Core is Provider-Agnostic**: Core contains zero provider-specific code, HTML parsers, or scrape bypass logic.
2. **Domain Service Responsibility**: Domain services (e.g., `CinemaService`) coordinate aggregation, normalization, deduplication, ranking, and resilience across registered plugins.
3. **Plugin Responsibility**: All provider-specific network requests, scraping, authentication, and translation into `plugin.v1` proto structures belong strictly inside the plugin binary.
4. **Resilience**: Slow, failing, or malicious plugins must not block or crash the overall user experience. Concurrent queries with strict deadlines guarantee responsiveness.

## Consequences
- Clean separation of concerns.
- Third-party updates only require releasing a plugin update rather than updating Core or native UI clients.
