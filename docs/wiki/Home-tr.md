> 🌐 **Diller:** [English](Home) · **Türkçe** · [Español](Home-es) · [Deutsch](Home-de) · [Français](Home-fr) · [Italiano](Home-it) · [Русский](Home-ru) · [日本語](Home-ja) · [Português](Home-pt) · [中文](Home-zh) · [العربية](Home-ar) · [فارسی](Home-fa)

# Vessel Wiki'ye Hoş Geldiniz

**Vessel**, baştan sona **Rust**, **Tauri v2** ve **Vue 3 / TypeScript** ile geliştirilmiş; **yerel-öncelikli (local-first)**, gizlilik odaklı, çok platformlu bir medya ve yayın merkezidir.

> *"Most ships dread Rust. This cargo vessel runs on it — shipped with Cargo, engineered not to rust, docking plugins by manifest at local ports."*

Merkezi sunuculara yaslanan uygulamaların aksine Vessel tüm bileşenleri doğrudan cihazınızda çalıştırır: izole QuickJS sandbox'ları içinde TypeScript/JavaScript eklentileri, `HMAC-SHA256` imzalı yerel `127.0.0.1` HTTP/HLS proxy'si, SQLite (`WAL` + `FTS5`) veritabanı, eşzamanlı AES-128 HLS & HTTP Range çevrimdışı indiricisi, IPTV + XMLTV EPG dizinleyici, çok kaynaklı **İntro/Özet/Kapanış/Önizleme Atlama** motoru ve harici **`mpv`** entegrasyonu.

---

## Temel Mimari İlkeler

1. **Yerel-Öncelikli & Sıfır Sunucu Maliyeti:** Hesap yok, bulut sunucusu yok, telemetri yok. Tüm çalışma zamanı verileriniz cihazınızdaki `<VesselRoot>` klasöründe tutulur.
2. **Değiştirilebilir Arayüz, Kalıcı Çekirdek:** Tüm iş mantığı arayüzden bağımsız Rust crate'lerinde (`vessel-engine` ve alt crate'ler) yer alır. Masaüstü (`Tauri v2`) ve gelecekteki `UniFFI` Android / Android TV arayüzleri ince kabuk olarak kalır.
3. **Kendi Kendine Yeten Eklentiler + Opsiyonel Metadata Zenginleştirme:** Eklentiler TMDB/IMDb/AniList gibi harici kimliklere ihtiyaç duymadan bağımsız çalışır; arka plan zenginleştirme katmanı ise kapak görselleri, bölüm başlıkları, atlama aralıkları (Skip) ve `(sezon, bölüm) ↔ mutlak bölüm` eşleştirmesi ekler.
4. **Varsayılan Alan Adı Güvenliği:** Eklentiler `32 MiB` heap / `512 KiB` stack ve `15 sn` zaman aşımı sınırlarıyla izole edilir; yerel diske okuyamaz ve DNS çözümlemesinden sonra SSRF koruması + kullanıcı izin penceresi (`Bir Kez İzin Ver` / `Her Zaman İzin Ver` / `Reddet`) üzerinden geçer.

---

## Türkçe Rehber Dizini

| Rehber | İçerik |
| :--- | :--- |
| **[Kurulum ve Derleme](Installation-and-Setup-tr)** | macOS (`.dmg`) & Windows (`.exe`) kurulumu, macOS `xattr -cr` Gatekeeper çözümü, Windows SmartScreen, `<VesselRoot>` dizin yapısı, GitHub güncelleyici ve kaynaktan derleme |
| **[Masaüstü Kullanım Rehberi](Desktop-App-User-Guide-tr)** | Hero Vitrin, **Akıllı İzlemeye Devam Et** & Manuel Düzenleyici, sağ tık menüsü, Arama filtreleri, **Eşleşmeyi Düzelt (Fix Match)**, Hızlı İzleme Durumu, `.backup.vessel` yedekleri |
| **[Oynatıcı, Skip & Çevrimdışı İndirmeler](Player-and-Downloads-tr)** | Tam ekran `/watch` oynatıcı, Ses parçaları, Altyazı özelleştirme & canlı önizleme, Klavye/Touchpad kontrolleri, **TheIntroDB/IntroDB/AniSkip** atlama, 4 seviyeli İndirme hiyerarşisi ve Sürükle-Bırak Kuyruk |
| **[Eklentiler, IPTV & SDK](Extensions-and-SDK-tr)** | `gh:`, `gl:`, kısa kodlar, Dev Mode kurulumu, Alan adı izinleri, M3U+/Xtream & XMLTV EPG, Canlı TV Eklentileri ve TypeScript Eklenti Geliştirme SDK'sı |

> Daha derin iç mimari ve API ayrıntıları için ana İngilizce teknik sayfalara da göz atabilirsiniz: [Architecture & Crates](Architecture-and-Crates) · [Plugin Development SDK](Plugin-Development-SDK) · [Metadata & Episode Sync](Metadata-and-Episode-Sync) · [Themes, i18n & Privacy](Themes-Localization-and-Privacy)

---

## Hızlı Özellik Özeti (`v0.1.46`)

- **Oynatma (`/watch`):** HLS (`hls.js`), MPEG-TS (`mpegts.js`), MP4/WebM, çoklu ses parçası seçimi, `.srt/.vtt/.ass/.ssa` altyazı & `-500ms / +500ms` senkronizasyon, touchpad kaydırma + **Doğal Kaydırma (Natural Scrolling)** desteği, **İntro / Özet / Kapanış / Önizleme Atla** (`Plugin > TheIntroDB > IntroDB > AniSkip`), doğrudan **Detaylara Git** butonu ve harici `mpv` aktarımı.
- **İzlemeye Devam Et:** Biten filmler ve sezon/dizi finalleri listeden otomatik kalkar, diziler otomatik bir sonraki bölüme ilerler; uzun bölüm başlıkları `...` ile dengelenir; kalem simgesi ile manuel bölüm ve ilerleme düzenleme (toplam süre seçilen bölümün gerçek süresinden salt okunur gelir).
- **4 Seviyeli Çevrimdışı İndirmeler:** **İçerik → Sezon → Bölüm → İndirilen Kaynaklar** ağacı (filmlerde doğrudan **İçerik → Kaynaklar**), çoklu iş parçacıklı HLS (`8` worker AES-128) ve HTTP Range indirmeleri, altyazı/ses dosyalarını birlikte indirme, anında duraklatma/silme iptali, sürükle-bırak kuyruk sıralama, indirilenlere özel İzlemeye Devam Et şeridi ve özel indirme konumu seçimi.
- **Kişiselleştirme:** **12 dil** (yerelleştirilmiş dil adlarıyla), **5 yerleşik tema** + `.theme.vessel` JSON içe aktarma, masaüstünde 6 menü düzeni (**Floating Bottom Dock** dahil), Android'de **Standart / Yüzen Alt Çubuk** seçeneği, özelleştirilebilir Hero ve afiş en-boy oranları, canlı altyazı önizleme kutusu ve özel CSS desteği.
- **Android Uygulaması (`apps/android`) & Gelişmiş Özellikler:** `vessel-ffi` + Jetpack Compose + Media3 ExoPlayer (yatay oynatıcı, Skip Intro/Outro), **6 kaynaklı Fix Match başlık araması** (`TMDB`, `IMDb`, `Simkl`, `AniList`, `MyAnimeList`, `Kitsu`), detay ekranı `⋮` menüsünden **Başka Kaynaktan Akış Dahil Et**, **İzlemeye Devam Et** hızlı düzenleme penceresi ve **Canlı TV** favorileri / sürükle-bırak sıralama / sınırsız katalog desteği.
