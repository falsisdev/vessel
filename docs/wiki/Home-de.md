> 🌐 **Sprachen:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · **Deutsch** · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Willkommen im Vessel-Wiki

**Vessel** ist ein **Local-First**, plattformübergreifendes Streaming- und Mediencenter, entwickelt mit **Rust**, **Tauri v2** und **Vue 3 / TypeScript**.

---

## Kernprinzipien

- **100 % Lokal & Ohne Server:** Keine Konten, keine Telemetrie. Erweiterungen, SQLite-Datenbank (`WAL` + `FTS5`), lokaler HTTP/HLS-Proxy (`127.0.0.1`), Offline-Downloads und Metadaten laufen komplett auf deinem Gerät.
- **Rust-Kern & Modulare UI:** Die gesamte Geschäftslogik steckt in UI-agnostischen Rust-Crates (`vessel-engine`), sodass Desktop und künftige Android-/Android-TV-Oberflächen (`UniFFI`) schlank bleiben.
- **Sandbox-Sicherheit:** TypeScript/JavaScript-Plugins laufen in isolierten QuickJS-Sandboxes (`32 MiB` Heap-Limit, `15s` Timeout) mit DNS-geprüftem SSRF-Schutz und Benutzerabfrage pro Domain.

---

## Hauptfunktionen (`v0.1.1-alpha.4`)

- **Vollbild-Player (`/watch`):** HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM und nahtlose Übergabe an externes **`mpv`** per JSON-IPC; Auswahl von **Audiospuren**, **Untertiteln** (`.srt/.vtt/.ass/.ssa`) inkl. Sync (`-500ms / +500ms`), Live-Vorschau im Einstellungsmenü, direkter **Zurück zu Details**-Button und automatisches **Intro / Rückblick / Abspann / Vorschau überspringen** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Intelligentes „Weiterschauen“:** Springt bei Serien automatisch zur nächsten Episode, blendet abgeschlossene Filme/Serienfinale aus, bietet Kontextmenü (Rechtsklick / langes Drücken) und einen manuellen Editor mit schreibgeschützter Episodendauer.
- **4-stufige Offline-Downloads:** Baumstruktur **Inhalt → Staffel → Episode → Streams** (bei Filmen direkt **Inhalt → Streams**), paralleler 8-Worker-HLS-AES-128- & HTTP-`Range`-Downloader inkl. Audio-/Untertitelspuren, sofortiges Pausieren/Löschen und sortierbare Download-Warteschlange per Drag & Drop.
- **Live-TV (IPTV & EPG):** M3U+ & Xtream Codes, XMLTV-EPG (**Jetzt / Nächste**), sofortige FTS5-Suche und eigener Tab für **Live-TV-Plugins**.
- **Anpassung:** **12 Sprachen**, **5 integrierte Themes** + `.theme.vessel`-Import, **6 Navigationslayouts** (inkl. *Floating Bottom Dock*) und portables `.backup.vessel`-Backup.

---

## Tastenkürzel & Touchpad-Gesten

| Aktion | Tastatur | Touchpad |
| :--- | :--- | :--- |
| Wiedergabe / Pause | `Leertaste` oder `K` | — |
| Zurück / Vor `10s` | `←` / `J` und `→` / `L` | Zwei-Finger horizontal wischen |
| Lautstärke `+5%` / `-5%` | `↑` / `↓` | Zwei-Finger vertikal wischen |
| Stumm / Vollbild | `M` / `F` | Pinch (`Ctrl + Scroll`) schaltet Fill/Fit um |

> **macOS-Tipp:** Aktiviere **Natürliches Touchpad-Scrollen** unter **Einstellungen → Wiedergabe & Datenschutz**, falls Wischgesten invertiert erscheinen.

---

## Schnellinstallation ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (Apple Silicon & Intel `.dmg`):** Ziehe `Vessel.app` in *Programme* und führe einmalig im Terminal aus:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** Bei SmartScreen-Warnung auf **Weitere Informationen → Trotzdem ausführen** klicken.

---

## Ausführliche Dokumentation (Englisch / Hauptdokumentation)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
