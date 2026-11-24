> 🌐 **English** · [Türkçe](Player-and-Downloads-tr) · [Wiki Home](Home)

# Player Controls & Skip Segments

Vessel's full-screen `/watch` player delivers a unified playback surface supporting **HLS (`.m3u8`)**, **MPEG-TS**, **MP4 / WebM**, local **Offline Streams**, **Multi-Audio**, styled **Subtitles**, **Touchpad Gestures**, **Automated Intro/Recap/Credits/Preview Skipping**, and external **`mpv`** handoff.

---

## 1. Playback HUD & Drawers

- **Auto-Hiding HUD:** Mouse movement reveals top/bottom overlays and automatically fades them away during uninterrupted playback.
- **Back to Details (`Details` Button):** Jump straight from an actively playing video to its Media Details page (and preserve/update progress cleanly).
- **Screen Lock:** Locks overlay controls, keyboard media keys, and touchpad gestures against accidental touches (`Esc` unlocks).
- **Episodes Drawer:** Browse seasons/episodes with thumbnails and descriptions directly inside the player and switch episodes without leaving `/watch`.
- **Sources Switcher:** Switch stream servers/qualities on the fly at your current timestamp.
- **Multi-Track Audio Switcher:** Automatically surfaces embedded multi-audio tracks exposed by HLS master manifests (`hls.js`) or native WebKit `<video>` audio tracks.
- **Playback Speed & Video Fit:** Adjust speed (`0.5x`, `0.75x`, `1x`, `1.25x`, `1.5x`, `2x`) and toggle aspect fit (`Contain` vs `Cover / Fill`).
- **External `mpv` Handoff:** One-click launch in native `mpv` via JSON IPC (`vessel-player-mpv`), automatically forwarding stream headers, active subtitles, and subtitle delay (`sub-delay`), while syncing `time-pos` and `eof` back into Vessel's SQLite progress!

---

## 2. Keyboard Shortcuts & Touchpad Gestures

### Keyboard Shortcuts (when unlocked and not typing in an input)

| Key | Action |
| :--- | :--- |
| `Space` or `K` | Play / Pause |
| `←` or `J` | Seek backward `-10s` |
| `→` or `L` | Seek forward `+10s` |
| `↑` / `↓` | Volume `+5%` / `-5%` |
| `M` | Mute / Unmute |
| `F` | Toggle Fullscreen |
| `Esc` | Close active drawer → Unlock screen → Exit fullscreen → Return |

### Touchpad Gestures
- **Two-Finger Horizontal Swipe:** Accumulated smooth seek (`« -15s` / `+15s »`) committed cleanly on gesture pause.
- **Two-Finger Vertical Swipe:** Smooth volume adjustment (`0% – 100%`) with on-screen HUD indicator.
- **Pinch (`Ctrl + Scroll`):** Toggles **Fill Screen (`cover`)** / **Fit Screen (`contain`)**.
- **Natural Touchpad Scrolling Toggle:** Available in **Settings → Playback & Privacy**. Enable this on macOS/trackpads using OS Natural Scrolling so swipe seek and volume directions match your intuition.

---

## 3. Subtitles, Sync & Live Preview

Inside the player's **Subtitles** drawer or **Settings → Subtitle Customization**:
- **Supported Formats:** `.srt`, `.vtt`, `.ass`, `.ssa` (stream-bundled, plugin-supplied, and offline-saved).
- **Live Subtitle Sync:** Adjust offset in **`-500ms` / `+500ms`** increments during playback.
- **Full Visual Customization (with Live Preview Box in Settings):**
  - Font Size (`px`)
  - Font Family
  - Text Color
  - Edge Style: **Outline**, **Shadow**, **Backdrop Box**, or **Clean (`None`)**
  - Backdrop Opacity (`0% – 100%`)
  - Bottom Position Offset (`px`)
  - Bold Toggle

---

## 4. Automated Skip Segments (`Intro`, `Recap`, `Credits`, `Preview`)

When watching Movies, Series, or Anime, Vessel resolves timeline skip intervals automatically and renders a **non-intrusive Skip button** when playback enters an active interval (with one-click dismiss per segment).

### Provider Priority Cascade
Vessel queries and merges segments in strict precedence order (`vessel-metadata/src/skip.rs`):

1. **Plugin / Stream Segments (Highest Priority):** Any `skipSegments` returned directly by the active extension or resolved stream.
2. **`TheIntroDB` (`api.theintrodb.org/v2`):** Supplies `intro`, `recap`, `credits`, and `preview` segments using `tmdb_id` or `imdb_id`.
3. **`IntroDB` (`api.introdb.app`):** Supplies `intro`, `recap`, and `outro` (`Credits`) via `imdb_id`, filtered by confidence threshold (`>= 0.40`).
4. **`AniSkip` (`api.aniskip.com/v2`):** Anime fallback using `mal_id` (supporting `op`/`mixed-op`, `ed`/`mixed-ed`, `recap`). If external `mal_id`, `imdb_id`, or `tmdb_id` is missing when `anilist_id` is known, Vessel resolves mappings via **`Ani.zip`** (`api.ani.zip/mappings`).

### Strict False-Positive Safety Guards
To avoid incorrect skip prompts:
- **AniSkip Season Guard:** AniSkip is disabled automatically when `season > 1` on series (since MAL assigns separate IDs per season, preventing Season 1 timings from triggering on Season 2+).
- **Episode Duration Mismatch Guard:** AniSkip entries whose `episodeLength` differs from the actual playing video duration by more than `120s` are discarded.
- **IntroDB Community Negative Flags:** If IntroDB explicitly marks `no_intro`, `no_recap`, or `no_outro`, AniSkip fallback will **not** override that denial.
- **Sanity Bounds Check (`sanitize_segment`):** Rejects zero/negative intervals, segments shorter than `3s`, intros/previews longer than `6 min`, recaps longer than `10 min`, segments covering `> 85%` of total video duration, or start times after `(duration - 1.5s)`.
- **Privacy Compliance:** Disabling external metadata enrichment in Settings (`metadataEnabled = false`) completely disables external skip-database queries while keeping plugin-supplied segments active.
