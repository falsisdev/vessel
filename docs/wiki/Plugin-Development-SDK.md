> 🌐 **English** · [Türkçe](Extensions-and-SDK-tr) · [Wiki Home](Home)

# Plugin Development SDK

Vessel plugins are written in **TypeScript** (`.ts`) or **JavaScript** (`.js` / `.mjs`) and executed inside isolated **QuickJS** sandboxes with **oxc** TypeScript compilation cached in SQLite. Plugins require **no Node.js runtime or build step** when loading a Dev Mode folder!

---

## 1. Sandbox Limits & Execution Model

| Resource | Default Limit |
| :--- | :--- |
| Heap Memory Limit | **32 MiB** per plugin sandbox (`DEFAULT_MEMORY_LIMIT_BYTES`) |
| Call Stack Limit | **512 KiB** (`DEFAULT_MAX_STACK_SIZE_BYTES`) |
| Execution Timeout | **15s** watchdog per call (`30s`+ extended allowance on stream resolution) |
| Warm LRU Pool | **18** active sandboxes kept warm (`5-minute` idle TTL) |
| Local Disk Access | **Disabled** (`fs.existsSync` -> `false`, `fs.readFileSync` throws) |
| ES6 Relative Imports | Supported for `.ts`, `.mts`, `.js`, `.mjs` inside the plugin package (`..` escape blocked) |

---

## 2. Minimal Directory Structure

```text
my-plugin/
├── manifest.json
├── index.ts
└── extractors/
    └── custom_host.ts   # Optional relative ES6 module
```

### `manifest.json` Schema (`schemaVersion: 1`)

```json
{
  "schemaVersion": 1,
  "id": "anthology",
  "name": "Anthology",
  "version": "1.0.0",
  "author": "Your Name",
  "description": "Catalog and stream resolver for Anthology.",
  "entry": "index.ts",
  "icon": "https://cdn.example.org/anthology.png",
  "minCoreVersion": "0.1.1-alpha.1",
  "capabilities": ["vod", "search", "subtitles"],
  "contentTypes": ["anime", "series"],
  "languages": ["en", "tr"],
  "domains": ["anthology.example", "cdn.anthology.example"],
  "allowAnyHttps": false,
  "isAdult": false,
  "files": ["extractors/custom_host.ts"],
  "settingsSchema": [
    {
      "key": "mirrorUrl",
      "label": "Mirror Base URL",
      "type": "text",
      "default": "https://anthology.example",
      "required": true,
      "placeholder": "https://anthology.example"
    },
    {
      "key": "apiKey",
      "label": "API Token",
      "type": "secret"
    },
    {
      "key": "preferDub",
      "label": "Prefer Dubbed Audio",
      "type": "toggle",
      "default": false
    },
    {
      "key": "quality",
      "label": "Default Quality",
      "type": "select",
      "default": "1080p",
      "options": [
        { "label": "1080p Full HD", "value": "1080p" },
        { "label": "720p HD", "value": "720p" }
      ]
    }
  ]
}
```

#### Allowed Values
- `capabilities` (non-empty): `"vod"`, `"search"`, `"live_tv"`, `"subtitles"`, `"extractor"`
- `contentTypes`: `"movie"`, `"series"`, `"anime"`, `"live"`
- `settingsSchema[].type`: `"text"`, `"secret"`, `"toggle"`, `"select"`

---

## 3. Repository (`repo.json`) & Shortcode Registry (`registry.json`)

### `repo.json`
```json
{
  "schemaVersion": 1,
  "id": "dev.example.main",
  "name": "Community Vessel Repo",
  "description": "Curated TypeScript plugins",
  "author": "Community Maintainer",
  "website": "https://github.com/example/vessel-repo",
  "iconUrl": "https://github.com/example/vessel-repo/raw/main/icon.png",
  "pluginLists": [],
  "plugins": [
    {
      "id": "anthology",
      "name": "Anthology",
      "version": "1.0.0",
      "description": "Catalog provider",
      "capabilities": ["vod", "search"],
      "contentTypes": ["anime", "series"],
      "languages": ["en"],
      "sourcePath": "plugins/anthology",
      "downloadUrl": "https://example.org/anthology.plugin.vessel",
      "sha256": "<optional-sha256-hex>"
    }
  ]
}
```

### `registry.json`
```json
{
  "schemaVersion": 1,
  "updatedAt": "2026-11-24T13:30:00Z",
  "codes": {
    "anthology": "gh:falsisdev/anthology",
    "tr": "gh:falsisdev/anthology"
  }
}
```

---

## 4. Provider Entry Methods (`index.ts`)

Export either `export default vessel.definePlugin({ ... })`, a plain default object, or named exports:

```typescript
export default vessel.definePlugin({
  async getHomeFeed(cursor?: string | null) {
    return {
      sections: [
        {
          id: "trending",
          title: "Trending",
          layout: "poster", // "poster" | "landscape"
          items: [
            {
              nativeId: "/anime/frieren",
              type: "series", // "movie" | "series" | "live" (NEVER "anime" here; set isAnime: true!)
              isAnime: true,
              title: "Frieren: Beyond Journey's End",
              year: 2023,
              poster: "https://...",
              rating: 9.1
            }
          ]
        }
      ],
      nextCursor: null
    };
  },

  async search(query: string, filters: Record<string, string>, page: number) {
    return {
      items: [],
      hasNextPage: false
    };
  },

  async getDetails(nativeId: string) {
    return {
      nativeId,
      type: "series", // "movie" | "series" | "live"
      isAnime: true,
      title: "Frieren: Beyond Journey's End",
      year: 2023,
      synopsis: "...",
      poster: "https://...",
      backdrop: "https://...",
      genres: ["Fantasy", "Adventure"],
      externalIds: {
        anilist: 154587, // Optional hints help Vessel's metadata/skip resolvers
        mal: 52991
      },
      // Optional lazy seasons array (triggers getSeasonEpisodes when expanding a season):
      // seasons: [{ id: "s1", season: 1, title: "Season 1", episodeCount: 28 }],
      episodes: [
        {
          streamId: "/watch/frieren-1",
          season: 1,
          episode: 1,
          absoluteEpisode: 1,
          title: "The Journey's End",
          overview: "...",
          thumbnail: "https://...",
          tags: ["sub"] // "sub" | "dub" | "raw"
        }
      ]
    };
  },

  // Optional: required only if getDetails returns `seasons` without full `episodes`
  async getSeasonEpisodes(nativeId: string, seasonId: string) {
    return [];
  },

  async getStreams(streamId: string) {
    return {
      streams: [
        {
          url: "https://cdn.anthology.example/ep1/master.m3u8",
          kind: "hls", // "hls" | "dash" | "mpeg_ts" | "mp4" | "mkv" | "webm" | "auto" | "embed"
          quality: "1080p",
          label: "Anthology CDN • 1080p",
          language: "ja",
          audioType: "sub", // "sub" | "dub" | "original"
          priority: 100,
          isDefault: true,
          flags: ["subtitled", "requires_proxy"], // "subtitled" | "dubbed" | "multi_audio" | "hdr" | "direct" | "requires_proxy"
          headers: {
            "Referer": "https://anthology.example/",
            "User-Agent": "Mozilla/5.0..."
          },
          needsExtractor: false,
          subtitles: [
            {
              source: { kind: "url", url: "https://cdn.anthology.example/ep1/en.vtt", headers: {} },
              lang: "en",
              label: "English",
              format: "vtt", // "srt" | "ass" | "ssa" | "vtt"
              isDefault: true
            }
          ],
          skipSegments: [
            {
              type: "intro", // "intro" | "recap" | "credits" | "preview"
              startMs: 85000,
              endMs: 175000
            }
          ]
        }
      ],
      subtitles: [] // Optional episode-level subtitles shared across streams
    };
  }
});
```

> **Important Schema Rule:** `type` (`MediaType`) is strictly structural: **`"movie"` | `"series"` | `"live"`**. Set **`"isAnime": true`** for anime titles so Vessel maps coordinates and routes AniList/AniZip/AniSkip cleanly!

---

## 5. Host SDK Reference (`vessel.*`)

### `vessel.http`
All requests execute asynchronously via Tokio + `reqwest` (`rustls`) and enforce SSRF & `DomainPolicy`:
- `await vessel.http.get(url, opts?)`
- `await vessel.http.post(url, body, opts?)`
- `await vessel.http.request({ method, url, body?, headers?, timeoutMs?, followRedirects? })`
- **Response object:** `{ status: number, url: string, headers: Record<string, string>, text(): string, json(): any, bytes(): Uint8Array }`

### `vessel.html`
Backed by Rust's native `scraper` HTML5 parser (far faster and lower memory than pure-JS DOM parsers):
- `const doc = vessel.html.parse(rawHtml)`
- Document methods: `doc.select(css)`, `doc.selectOne(css)`, `doc.html()`
- Element methods: `el.tagName()`, `el.text()`, `el.ownText()`, `el.html()`, `el.outerHtml()`, `el.attr(name)`, `el.attrs()`, `el.hasClass(name)`, `el.matches(css)`, `el.select(css)`, `el.selectOne(css)`, `el.children()`, `el.parent()`, `el.next()`, `el.prev()`

### `vessel.crypto`
Native Rust implementations:
- `base64Encode(input)`, `base64Decode(b64)`, `base64DecodeBytes(b64)`
- `hexEncode(input)`, `hexDecode(hex)`
- `md5(str)`, `sha1(str)`, `sha256(str)`, `hmacSha256(key, msg)`
- `aesEncrypt(plain, key, iv?, mode?)`, `aesDecrypt(cipher, key, iv?, mode?)` (`cbc`, `ctr`, `ecb`, `gcm`)
- `rc4(data, key)`

### `vessel.storage`
Isolated per-plugin persistent SQLite KV store & user settings reader:
- `await vessel.storage.get(key)`
- `await vessel.storage.set(key, value)`
- `await vessel.storage.remove(key)`
- `await vessel.storage.getSetting(key)` *(reads `settingsSchema` values configured by the user)*

### `vessel.unpack` & `vessel.evalWithEnv`
- `vessel.unpack(packedJs)` — Unpacks Dean Edwards `eval(function(p,a,c,k,e,d){...})` obfuscated embed scripts cleanly in Rust.
- `await vessel.evalWithEnv(script, { url?, userAgent?, cookies? })` — Evaluates isolated embed scripts inside a minimal ephemeral sandbox with fake location/UA/cookie environment.

### Built-in Web & Nuvio Compatibility Shims
Available automatically inside every plugin without extra imports:
- **Web APIs:** `fetch()`, `URL`, `URLSearchParams`, `TextEncoder`, `TextDecoder`, `atob`, `btoa`, `setTimeout`, `clearTimeout`, `AbortController`, `AbortSignal`, `console.{log,info,warn,error,debug}` (emits `PluginLog` events)
- **CommonJS `require(...)` shims:** `cheerio` (`cheerio-without-node-native`, `react-native-cheerio`), `crypto-js` (`globalThis.CryptoJS`), `axios`, `path`, and stubbed `fs` (always reports missing files for sandbox safety).

---

## 6. Testing in Dev Mode & Packing `.plugin.vessel`

1. Put `manifest.json` + `index.ts` in a folder (e.g. `~/Plugins/anthology`).
2. Open Vessel → **Extensions → Inspect Source**, paste `/Users/you/Plugins/anthology`, and install!
3. Click refresh/reload after editing your TypeScript files—Vessel strips types on the fly via `oxc`.
4. To distribute a standalone package, zip the contents (`manifest.json` at the archive root alongside `index.ts` and relative modules) into **`my-plugin.plugin.vessel`** (`.zip` container, optional `sha256` verification in `repo.json`).
