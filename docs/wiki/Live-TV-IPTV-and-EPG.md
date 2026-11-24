> 🌐 **English** · [Türkçe](Extensions-and-SDK-tr) · [Wiki Home](Home)

# Live TV, IPTV & EPG Guide

Vessel supports two complementary ways to watch live channels in **Live TV (`IptvView`)**: standard **M3U+ / Xtream Codes** profiles and dedicated **Live TV Plugins**.

---

## 1. M3U+ & Xtream Codes Profiles (`vessel-iptv`)

### Adding an IPTV Profile
Open **Live TV → M3U / Xtream → Add IPTV Profile**:
- **M3U / M3U+ Playlist URL:** Parses `#EXTINF` metadata (`tvg-id`, `tvg-name`, `tvg-logo`, `group-title`) and playlist user-agent hints. Generates deterministic channel IDs so favorites survive re-syncs!
- **Xtream Codes (`Server + Username + Password`):** Automatically fetches live categories, live streams, and resolves the companion `xmltv.php` EPG feed URL when no custom XMLTV URL is supplied.
- **Custom `User-Agent`:** Configure a per-profile `User-Agent` header applied seamlessly by `vessel-proxy` when streaming channels.

### Instant Search, Favorites & Groups
- **SQLite FTS5 Search:** Channels are indexed in an FTS5 full-text table for sub-millisecond filtering across thousands of channels.
- **Group Filter & Favorites Only:** Toggle between channel groups or restrict view to starred favorites (`Favorites` persist across playlist re-syncs!).

### XMLTV Electronic Program Guide (EPG)
- Click **Sync EPG** to fetch and parse XMLTV programme schedules (`<programme>`, timezone offsets, entity decoding).
- Channel cards display live **Now** and **Next** programme titles and time ranges directly inside the grid.

---

## 2. Dedicated Live TV Plugins Tab

Switch to **Live TV → Live TV Plugins** to browse live channels supplied directly by installed plugins carrying the `live_tv` capability (or `Live` content category):
- Supports live **HLS (`.m3u8`)** and raw **MPEG-TS (`.ts`)** streams natively via `hls.js` and `mpegts.js`, routed transparently through Vessel's loopback Smart Proxy whenever custom `Referer`, `Origin`, or `User-Agent` headers are required.
- Channels can be filtered by name or opened directly into the `/watch` player (Live mode automatically hides seek/duration bars and switches cleanly between channels).
