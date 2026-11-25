> 🌐 **اللغات:** [English](Home) · [Türkçe](Home-tr) · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · **العربية** · [فارسی](Home-fa)

# مرحبًا بك في ويكي Vessel

**Vessel** هو مركز وسائط وبث متعدد المنصات يعتمد مبدأ **المحلية أولًا (Local-First)** والخصوصية التامة، مبني بالكامل بلغة **Rust** وإطار **Tauri v2** و **Vue 3 / TypeScript**.

---

## المبادئ الأساسية

- **تشغيل محلي بالكامل بدون خوادم:** بلا حسابات وبلا تتبع. تعمل الإضافات وقاعدة بيانات SQLite (`WAL` + `FTS5`) والوكيل المحلي HTTP/HLS (`127.0.0.1`) والتنزيلات والبيانات الوصفية مباشرة على جهازك.
- **نواة Rust + واجهة مرنة:** يعيش منطق التطبيق بالكامل داخل حزم Rust (`vessel-engine`) لتبقى واجهات سطح المكتب ولاحقًا Android / Android TV (`UniFFI`) خفيفة وسريعة.
- **عزل وأمان النطاقات:** تعمل إضافات TypeScript/JavaScript داخل بيئة QuickJS معزولة (حد ذاكرة `32 MiB` ومهلة `15 ثانية`) مع حماية SSRF وطلب إذن المستخدم قبل الاتصال بأي نطاق خارجي.

---

## الميزات الرئيسية (`v0.1.45`)

- **مشغّل ملء الشاشة (`/watch`):** دعم HLS (`hls.js`) و MPEG-TS (`mpegts.js`) و MP4/WebM والتمرير إلى مشغّل **`mpv`** الخارجي عبر JSON IPC؛ تبديل **المسارات الصوتية**، و**الترجمة** (`.srt/.vtt/.ass/.ssa`) مع مزامنة (`-500ms / +500ms`) ومعاينة فورية، وزر **الانتقال للتفاصيل**، وتخطي تلقائي للمقدمة والملخص والاعتمادات (`Plugin > TheIntroDB > IntroDB > AniSkip`).
- **متابعة المشاهدة الذكية:** انتقال تلقائي للحلقة التالية عند إتمام الحلقة، إخفاء الأفلام والنهايات المكتملة، قائمة سياق بالزر الأيمن أو الضغط المطول، ومحرر يدوي.
- **تنزيلات بدون إنترنت بـ 4 مستويات:** تنظيم شجري **العمل → الموسم → الحلقة → المصادر** (و **العمل → المصادر** للأفلام)، تنزيل متوازٍ لـ HLS (`8` خيوط مع فك تشفير AES-128) و HTTP `Range` مع الصوت والترجمة، إيقاف مؤقت/حذف فوري، وإعادة ترتيب القائمة بالسحب والإفلات.
- **البث المباشر (IPTV و EPG):** ملفات M3U+ و Xtream Codes ودليل برامج XMLTV وبحث SQLite FTS5 فوري وتبويب خاص لإضافات البث المباشر.
- **التخصيص:** **12 لغة**، و**5 سمات مدمجة** مع دعم `.theme.vessel`، و**6 تخطيطات تنقل** ونسخ احتياطي `.backup.vessel`.

---

## اختصارات لوحة المفاتيح ولوحة اللمس

| الإجراء | لوحة المفاتيح | لوحة اللمس |
| :--- | :--- | :--- |
| تشغيل / إيقاف مؤقت | `Space` أو `K` | — |
| تقديم / ترجيع `10 ثوانٍ` | `←` / `J` و `→` / `L` | سحب أفقي بإصبعين |
| مستوى الصوت `+5%` / `-5%` | `↑` / `↓` | سحب عمودي بإصبعين |
| كتم الصوت / ملء الشاشة | `M` / `F` | الضم (`Ctrl + Scroll`) لتغيير الاحتواء |

---

## التثبيت السريع ([Releases](https://github.com/falsisdev/vessel/releases))

- **macOS (`.dmg` Apple Silicon & Intel):** اسحب `Vessel.app` إلى *التطبيقات* ثم نفّذ مرة واحدة في Terminal:
  ```bash
  xattr -cr /Applications/Vessel.app
  ```
- **Windows (`-setup.exe`):** إذا ظهر تنبيه SmartScreen، اختر **معلومات إضافية ← التشغيل على أي حال**.

---

## الوثائق التفصيلية الكاملة (بالإنجليزية)

- [Installation & Setup](Installation-and-Setup) · [Desktop App User Guide](Desktop-App-User-Guide) · [Player & Skip Segments](Player-Controls-and-Skip-Segments) · [Offline Downloads & Queue](Offline-Downloads-and-Queue) · [Live TV, IPTV & EPG](Live-TV-IPTV-and-EPG) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Extensions & Repositories](Extensions-and-Repositories) · [Plugin Development SDK](Plugin-Development-SDK) · [Architecture & Crates](Architecture-and-Crates) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)
