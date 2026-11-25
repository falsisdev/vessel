> 🌐 **Idiomas:** [English](Home) · [Türkçe](Home-tr) · **Español** · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Bienvenido a la Wiki de Vessel

**Vessel** es un centro multimedia y de streaming **local-first**, multiplataforma y respetuoso con la privacidad, desarrollado en **Rust**, **Tauri v2** y **Vue 3 / TypeScript**.

---

## Principios Fundamentales

- **100 % Local y Sin Servidores:** Sin cuentas ni telemetría. Los complementos, la base de datos SQLite (`WAL` + `FTS5`), el proxy HTTP/HLS (`127.0.0.1`), las descargas sin conexión y los metadatos se ejecutan directamente en tu dispositivo.
- **Motor en Rust + Interfaz Modular:** Toda la lógica reside en los crates de Rust (`vessel-engine`) para mantener la interfaz ágil en Escritorio y futuros clientes Android / Android TV (`UniFFI`).
- **Seguridad por Dominios:** Los complementos TypeScript/JavaScript corren en sandboxes QuickJS aislados (`32 MiB` RAM, límite de `15s`) con protección SSRF y autorización de dominios controlada por el usuario.

---

## Características Principales (`v0.1.44`)

- **Reproductor `/watch` estilo CloudStream:** Soporte nativo para HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM y envío externo a **`mpv`** vía JSON IPC; cambio de pista de **Audio**, **Subtítulos** (`.srt/.vtt/.ass/.ssa`) con sincronización (`-500ms / +500ms`), estilo personalizable con vista previa, botón directo **Ir a Detalles** y salto automático de **Intro / Resumen / Créditos / Avance** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Seguir Viendo Inteligente:** Avanza automáticamente al siguiente episodio tras terminar uno, oculta películas/finales completados, incluye menú contextual (clic derecho / pulsación larga) y un editor manual con duración real de episodios.
- **Descargas Sin Conexión en 4 Niveles:** Árbol jerárquico **Contenido → Temporada → Episodio → Fuentes** (y **Contenido → Fuentes** en películas), descarga HLS paralela (`8` hilos AES-128) + HTTP `Range`, pistas de audio y subtítulos incluidos, pausa/eliminación instantánea y cola arrastrable.
- **TV en Vivo (IPTV + EPG):** Listas M3U+ y Xtream Codes con guía XMLTV (**Ahora / Siguiente**), búsqueda instantánea SQLite FTS5 y pestaña dedicada de **Plugins Live TV**.
- **Personalización:** **12 idiomas** integrados, **5 temas** + importación `.theme.vessel`, **6 diseños de navegación** (incluido *Floating Bottom Dock*) y copias de seguridad `.backup.vessel`.

---

## Atajos y Gestos del Reproductor

| Acción | Teclado | Panel Táctil (Touchpad) |
| :--- | :--- | :--- |
| Reproducir / Pausar | `Espacio` o `K` | — |
| Retroceder / Avanzar 10 s | `←` / `J` y `→` / `L` | Deslizar horizontal con dos dedos |
| Volumen `+5%` / `-5%` | `↑` / `↓` | Deslizar vertical con dos dedos |
| Silenciar / Pantalla Completa | `M` / `F` | Pellizco (`Ctrl + Scroll`) ajusta relleno |

> Activa **Desplazamiento natural del touchpad** en **Ajustes → Reproducción y privacidad** si la dirección horizontal o vertical se siente invertida en macOS.

---

## Instalación Rápida ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (Apple Silicon & Intel `.dmg`):** Arrastra `Vessel.app` a Aplicaciones y ejecuta una vez en Terminal para quitar la cuarentena de Gatekeeper:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** Si Windows SmartScreen muestra un aviso, pulsa **Más información → Ejecutar de todas formas**.

---

## Documentación Detallada (Inglés / Principal)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
