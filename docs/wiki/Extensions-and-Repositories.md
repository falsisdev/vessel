> 🌐 **English** · [Türkçe](Extensions-and-SDK-tr) · [Wiki Home](Home)

# Extensions & Repositories Guide

Vessel extensions run inside isolated QuickJS sandboxes (`vessel-plugin-rt`) and can be installed from repositories, Git hosts, shortcodes, deep links, or local folders.

---

## 1. Supported Install Sources (`SourceResolver`)

In **Extensions → Inspect Source**, paste any of the following formats:

| Format | Example | Resolution Behavior |
| :--- | :--- | :--- |
| **Direct HTTPS URL** | `https://example.org/repo.json` or `...plugin.vessel` | Resolves `repo.json` repository indexes, single `manifest.json`, or signed `.plugin.vessel` zip packages |
| **GitHub Shorthand (`gh:`)** | `gh:owner/repo` / `gh:owner/repo@v2/plugins/foo` | Checks `main`/`master` or specified branch/tag for `repo.json`, `manifest.json`, or relative TS/JS files |
| **GitLab Shorthand (`gl:`)** | `gl:group/project[@ref][/subpath]` | Resolves GitLab repositories cleanly across branches/subpaths |
| **Shortcodes (`registry.json`)** | `anthology`, `tr` | Looks up short names via Vessel's default [`registry.json`](https://github.com/falsisdev/vessel/blob/main/registry.json) (e.g. `anthology` & `tr` → `gh:falsisdev/anthology`, with offline fallback) or HTTP `301/302` redirect shorteners (up to 3 hops) |
| **Deep Links (`vessel://install`)** | `vessel://install?src=gh:owner/repo` | Parses deep-link install URI directly into source resolver |
| **Local Dev Mode Folder** | `/Users/you/dev/my-vessel-plugin` | Loads `manifest.json` + local `.ts`/`.js` ES6 imports straight from disk for instant hot testing |
| **Nuvio Scraper Manifests** | `https://.../providers.json` or `nuvio:...` | Wraps compatible open-source JS/TS Nuvio scraper packages automatically |

> **Security Policy:** Obfuscated/packed closed-source plugin bundles and compiled Android DEX (`CloudStream .cs3`) binaries are rejected by `vessel-plugin-rt` compatibility checks.

---

## 2. Trust Badges (`Official` vs `Community`)

Vessel ignores any `"badge"` field claimed inside `manifest.json`. Trust badges are strictly assigned by `vessel-plugin-rt`'s `SourceResolver` (`SourceTrustBadge::Official` vs `Community`) based solely on whether a source originated from verified official repositories/signatures.

---

## 3. Domain Policy, SSRF Protection & Permission Prompts

Every outbound HTTP request issued by a plugin (`vessel.http.*`, `fetch`, or `axios`) passes through `DomainPolicy`:
1. **SSRF Hard Block (Zero Override):** Private/loopback/link-local/multicast IPs (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `::1`, `fc00::/7`, etc.) are blocked **even after DNS resolution** and even if `allowAnyHttps: true` is declared! Cross-host redirects are re-validated hop by hop (`Policy::none()`).
2. **Manifest Allowlist:** Domains declared in `manifest.json` (`"domains": ["example.com"]`) are authorized immediately (exact host or subdomain match).
3. **Interactive Domain Permission Prompt:** If a plugin contacts an undeclared external host (e.g. a new CDN or video host), execution pauses safely and displays the **Plugin Domain Permission Request** overlay with three choices:
   - **Allow Once** (session grant)
   - **Always Allow** (persisted in SQLite `domain_grants`)
   - **Reject** (returns `[VesselDomainBlockedError]` to the plugin)

You can inspect or revoke granted domains anytime inside **Extensions → Allowed Domains**.

---

## 4. Per-Plugin `settingsSchema` & Secrets

Plugins can declare dynamic forms via `settingsSchema` (`text`, `secret`, `toggle`, `select`).
- Clicking **Plugin Settings** opens an auto-generated UI form for that extension.
- Fields with `"type": "secret"` are stored through `SecretStore` and **excluded by default** from `.backup.vessel` exports unless the user explicitly enables *"Include Stored Secrets in Export"*.
- Plugins read configured values asynchronously via `await vessel.storage.getSetting(key)`.
