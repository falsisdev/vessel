> 🌐 **زبان‌ها:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · **فارسی**

# به ویکی Vessel خوش آمدید

**Vessel** یک مرکز رسانه و استریم چندسکویی و **محلی‌محور (Local-First)** با تمرکز بر حریم خصوصی است که از پایه با **Rust**، **Tauri v2** و **Vue 3 / TypeScript** ساخته شده است.

---

## اصول کلیدی

- **کاملاً محلی و بدون نیاز به سرور:** بدون حساب کاربری و بدون ردیابی. افزونه‌ها، پایگاه‌داده SQLite (`WAL` + `FTS5`)، پروکسی محلی HTTP/HLS (`127.0.0.1`)، دانلودهای آفلاین و فراداده‌ها همگی روی دستگاه شما اجرا می‌شوند.
- **هسته Rust + رابط ماژولار:** تمام منطق برنامه در کریت‌های Rust (`vessel-engine`) قرار دارد تا کلاینت دسکتاپ و نسخه‌های آینده Android / Android TV (`UniFFI`) سبک و سریع بمانند.
- **امنیت دامنه:** افزونه‌های TypeScript/JavaScript درون سندباکس ایزوله QuickJS (حافظه `32 MiB` و زمان‌سنج `15 ثانیه`) همراه با محافظت SSRF و پنجره اجازه کاربر اجرا می‌شوند.

---

## قابلیت‌های اصلی (`v0.1.44`)

- **پخش‌کننده تمام‌صفحه (`/watch`):** پشتیبانی از HLS (`hls.js`)، MPEG-TS (`mpegts.js`)، MP4/WebM و ارسال به **`mpv`** خارجی با JSON IPC؛ تغییر **تراک صوتی**، **زیرنویس** (`.srt/.vtt/.ass/.ssa`) با همگام‌سازی (`-500ms / +500ms`) و پیش‌نمایش زنده، دکمه **رفتن به جزئیات** و رد خودکار **تیتراژ / خلاصه / پایان‌بندی** (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **ادامه تماشای هوشمند:** انتقال خودکار به قسمت بعدی پس از پایان قسمت، پنهان‌سازی فیلم‌ها و قسمت‌های پایانی، منوی کلیک راست / لمس طولانی و ویرایشگر دستی.
- **دانلودهای آفلاین ۴ سطحی:** ساختار درختی **عنوان ← فصل ← قسمت ← منبع** (و **عنوان ← منبع** برای فیلم‌ها)، دانلود هم‌زمان HLS با `8` ورکر رمزگشایی AES-128 و HTTP `Range` به‌همراه صدا و زیرنویس، توقف/حذف آنی و صف دانلود قابل کشیدن و مرتب‌سازی.
- **تلویزیون زنده (IPTV و EPG):** فهرست‌های M3U+ و Xtream Codes، راهنمای برنامه XMLTV، جستجوی آنی SQLite FTS5 و تب اختصاصی **افزونه‌های Live TV**.
- **شخصی‌سازی:** **۱۲ زبان داخلی**، **۵ تم آماده** + فایل `.theme.vessel`، **۶ چیدمان ناوبری** و پشتیبان‌گیری `.backup.vessel`.

---

## میانبرهای صفحه‌کلید و تاچ‌پد

| عملکرد | صفحه‌کلید | تاچ‌پد |
| :--- | :--- | :--- |
| پخش / توقف | `Space` یا `K` | — |
| عقب / جلو `۱۰ ثانیه` | `←` / `J` و `→` / `L` | کشیدن افقی دو انگشتی |
| صدا `+5%` / `-5%` | `↑` / `↓` | کشیدن عمودی دو انگشتی |
| بی‌صدا / تمام‌صفحه | `M` / `F` | نیشگون (`Ctrl + Scroll`) برای Fit/Fill |

---

## نصب سریع ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon & Intel):** برنامه `Vessel.app` را به *Applications* بکشید و یک بار در Terminal اجرا کنید:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** در پیام SmartScreen روی **More info → Run anyway** کلیک کنید.

---

## مستندات کامل فنی (انگلیسی)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
