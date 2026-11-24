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

Switch to **Live TV → Live TV Plugins** to browse live channels supplied directly by installed plugins carrying the `live_tv` capability (or live categories):
- **Isolated from Home:** Live TV plugin catalogs are kept out of main Home discovery feeds so Home stays focused on VOD movies, series, and anime.
- **Persistent Favorites Bar:** Click the **Star (`★`)** icon on any plugin live channel to pin it to the top **Favorites** shelf when opening Live TV.
- **View Modes & Custom Ordering:** Switch between **Grid**, **Compact List**, and **Large Tiles** views, and **drag-and-drop** plugin groups/categories to prioritize your preferred categories (e.g., placing sports channels first).
- **Uncapped Catalog Listing:** Displays full channel lists exposed by live plugins without artificial per-catalog item caps.
- **Direct-to-Player Live Playback:** Clicking a live channel opens the stream directly in the player via **HLS (`.m3u8`)** or **MPEG-TS (`.ts`)** (routed transparently through Smart Proxy for custom headers) without an intermediate Details view, audio-track drawer, or offline download buttons.
