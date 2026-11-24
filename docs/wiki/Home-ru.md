> 🌐 **Языки:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · **Русский** · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Добро пожаловать в вики Vessel

**Vessel** — это кроссплатформенный медиацентр и стриминговый плеер с архитектурой **local-first**, написанный с нуля на **Rust**, **Tauri v2** и **Vue 3 / TypeScript**.

---

## Основные принципы

- **100 % локально и без серверов:** Никаких аккаунтов и телеметрии. Плагины, база SQLite (`WAL` + `FTS5`), локальный HTTP/HLS-прокси (`127.0.0.1`), офлайн-загрузки и метаданные работают прямо на вашем устройстве.
- **Ядро на Rust и модульный UI:** Вся логика сосредоточена в крейтах Rust (`vessel-engine`), что позволяет легко поддерживать Desktop и будущие приложения для Android / Android TV (`UniFFI`).
- **Изоляция и безопасность:** Плагины на TypeScript/JavaScript запускаются в изолированных песочницах QuickJS (`32 МиБ` ОЗУ, таймаут `15 с`) с проверкой SSRF после DNS-резолвинга и запросом разрешений для доменов.

---

## Ключевые возможности (`v0.1.3`)

- **Полноэкранный плеер (`/watch`):** Поддержка HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM и передача во внешний **`mpv`** по JSON IPC; переключение **аудиодорожек**, **субтитры** (`.srt/.vtt/.ass/.ssa`) с синхронизацией (`-500 мс / +500 мс`) и предпросмотром, переход обратно на **страницу контента** и пропуск **интро / рекапа / титров / превью** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **Умный раздел «Продолжить просмотр»:** Автоматически переключается на следующую серию после завершения текущей, скрывает завершённые фильмы и финалы сериалов, имеет контекстное меню (ПКМ / долгое нажатие) и ручной редактор.
- **4-уровневая структура офлайн-загрузок:** **Контент → Сезон → Серия → Источники** (для фильмов — **Контент → Источники**), многопоточная загрузка HLS (`8` воркеров AES-128) и HTTP `Range` вместе с аудио и субтитрами, мгновенная пауза/удаление и перетаскиваемая очередь.
- **IPTV и EPG:** Плейлисты M3U+ и Xtream Codes, программа передач XMLTV (**Сейчас / Далее**), мгновенный поиск SQLite FTS5 и отдельная вкладка **Плагины Live TV**.
- **Настройка:** **12 языков**, **5 встроенных тем** + импорт `.theme.vessel`, **6 вариантов навигации** и резервные копии `.backup.vessel`.

---

## Горячие клавиши и жесты тачпада

| Действие | Клавиатура | Тачпад |
| :--- | :--- | :--- |
| Пауза / Воспроизведение | `Пробел` или `K` | — |
| Перемотка `±10 с` | `←` / `J` и `→` / `L` | Горизонтальный свайп двумя пальцами |
| Громкость `±5%` | `↑` / `↓` | Вертикальный свайп двумя пальцами |
| Звук / Полный экран | `M` / `F` | Щипок (`Ctrl + прокрутка`) меняет Fit/Fill |

---

## Быстрая установка ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (Apple Silicon & Intel `.dmg`):** Перетащите `Vessel.app` в *Программы* и выполните один раз в Терминале:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** Если SmartScreen заблокирует установщик, выберите **Подробнее → Выполнить в любом случае**.

---

## Подробная документация (на английском языке)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
