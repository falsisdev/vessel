> 🌐 **Langues :** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · **Français** · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Bienvenue sur le Wiki de Vessel

**Vessel** est un centre multimédia et de streaming **local-first**, multiplateforme et respectueux de la vie privée, écrit en **Rust**, **Tauri v2** et **Vue 3 / TypeScript**.

---

## Principes Clés

- **100 % Local & Sans Serveur :** Aucun compte ni télémétrie. L'exécution des extensions, la base SQLite (`WAL` + `FTS5`), le proxy local HTTP/HLS (`127.0.0.1`), les téléchargements hors ligne et les métadonnées restent sur votre appareil.
- **Cœur Rust & Interface Modulaire :** Toute la logique réside dans des crates Rust agnostiques (`vessel-engine`), gardant les clients Desktop et futurs Android / Android TV (`UniFFI`) légers.
- **Sécurité par Domaine :** Chaque extension TypeScript/JavaScript tourne dans un bac à sable QuickJS isolé (`32 Mio` de RAM, limite `15 s`) avec protection SSRF et invite d'autorisation par domaine.

---

## Fonctionnalités Principales (`v0.1.46`)

- **Lecteur Plein Écran (`/watch`) :** Lecture HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM et transfert vers **`mpv`** via JSON IPC ; sélecteur de **Pistes Audio**, **Sous-titres** (`.srt/.vtt/.ass/.ssa`) avec synchronisation (`-500 ms / +500 ms`), aperçu en direct, bouton **Aller aux Détails** et saut automatique **Intro / Récap / Générique / Aperçu** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Reprendre la Lecture Intelligent :** Passe automatiquement à l'épisode suivant après un épisode terminé, masque les films/fins de série achevés, propose un menu contextuel (clic droit / appui long) et un éditeur manuel.
- **Téléchargements Hors Ligne à 4 Niveaux :** Hiérarchie **Contenu → Saison → Épisode → Flux** (et **Contenu → Flux** pour les films), téléchargement HLS parallèle (`8` workers AES-128) + HTTP `Range`, audio et sous-titres inclus, pause/suppression instantanée et file d'attente réorganisable.
- **TV en Direct (IPTV & EPG) :** Profils M3U+ et Xtream Codes, guide XMLTV (**En cours / À suivre**), recherche SQLite FTS5 et onglet **Extensions TV en Direct**.
- **Personnalisation :** **12 langues**, **5 thèmes** + import `.theme.vessel`, **6 dispositions** (dont *Floating Bottom Dock*) et sauvegardes `.backup.vessel`.

---

## Raccourcis & Gestes Trackpad

| Action | Clavier | Trackpad |
| :--- | :--- | :--- |
| Lecture / Pause | `Espace` ou `K` | — |
| Avancer / Reculer `10s` | `→` / `L` et `←` / `J` | Balayage horizontal à deux doigts |
| Volume `+5%` / `-5%` | `↑` / `↓` | Balayage vertical à deux doigts |
| Muet / Plein écran | `M` / `F` | Pincement (`Ctrl + Molette`) ajuste l'écran |

> Activez **Défilement naturel du trackpad** dans **Paramètres → Lecture & Confidentialité** si la direction est inversée sous macOS.

---

## Installation Rapide ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon & Intel) :** Glissez `Vessel.app` dans *Applications* puis exécutez une fois dans le Terminal :
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`) :** Sous SmartScreen, cliquez sur **Informations complémentaires → Exécuter quand même**.

---

## Documentation Complète (Anglais / Référence Principale)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
