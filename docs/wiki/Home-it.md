> 🌐 **Lingue:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · **Italiano** · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Benvenuto nella Wiki di Vessel

**Vessel** è un media center e client di streaming **local-first**, multipiattaforma e orientato alla privacy, realizzato in **Rust**, **Tauri v2** e **Vue 3 / TypeScript**.

---

## Principi Chiave

- **100 % Locale e Zero Server:** Nessun account o telemetria. Estensioni, database SQLite (`WAL` + `FTS5`), proxy HTTP/HLS locale (`127.0.0.1`), download offline e metadati girano sul tuo dispositivo.
- **Motore Rust & Interfaccia Modulare:** Tutta la logica applicativa vive nei crate Rust (`vessel-engine`), mantenendo snelle le interfacce Desktop e Android / Android TV (`UniFFI`).
- **Sicurezza per Dominio:** I plugin TypeScript/JavaScript vengono eseguiti in sandbox QuickJS isolate (`32 MiB` RAM, timeout `15 s`) con protezione SSRF e permessi dominio per l'utente.

---

## Funzionalità Principali (`v0.1.46`)

- **Player `/watch` in Stile CloudStream:** Supporto per HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM e invio a **`mpv`** esterno via JSON IPC; selettore **Tracce Audio**, **Sottotitoli** (`.srt/.vtt/.ass/.ssa`) con sync (`-500 ms / +500 ms`), anteprima dal vivo, pulsante diretto **Dettagli** e salto automatico **Intro / Riepilogo / Titoli di coda / Anteprima** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Continua a Guardare Intelligente:** Avanza all'episodio successivo al completamento, nasconde film/finali terminati, offre menu contestuale (clic destro / pressione prolungata) ed editor manuale.
- **Download Offline a 4 Livelli:** Struttura ad albero **Contenuto → Stagione → Episodio → Flussi** (e **Contenuto → Flussi** per i film), download paralleli HLS AES-128 a `8` worker + HTTP `Range`, audio/sottotitoli inclusi, pausa/eliminazione istantanea e coda riordinabile.
- **Live TV (IPTV & EPG):** Liste M3U+ e Xtream Codes, guida XMLTV (**Ora / Successivo**), ricerca istantanea FTS5 e scheda dedicata **Plugin Live TV**.
- **Personalizzazione:** **12 lingue**, **5 temi** + import `.theme.vessel`, **6 layout di navigazione** e backup `.backup.vessel`.

---

## Scorciatoie e Gesti Touchpad

| Azione | Tastiera | Touchpad |
| :--- | :--- | :--- |
| Riproduci / Pausa | `Spazio` o `K` | — |
| Avanti / Indietro `10s` | `→` / `L` e `←` / `J` | Scorrimento orizzontale a due dita |
| Volume `+5%` / `-5%` | `↑` / `↓` | Scorrimento verticale a due dita |
| Muto / Schermo intero | `M` / `F` | Pinch (`Ctrl + Scroll`) adatta/riempi |

---

## Installazione Rapida ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon & Intel):** Trascina `Vessel.app` in *Applicazioni* ed esegui una volta nel Terminale:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** In caso di avviso SmartScreen, clicca **Ulteriori informazioni → Esegui comunque**.

---

## Documentazione Completa (Inglese / Riferimento Principale)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
