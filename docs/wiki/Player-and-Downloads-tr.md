> 🌐 [English](Player-Controls-and-Skip-Segments) · [Offline Downloads (EN)](Offline-Downloads-and-Queue) · **Türkçe** · [Türkçe Ana Sayfa](Home-tr)

# Oynatıcı, Skip Sistemi ve Çevrimdışı İndirmeler

---

## 1. Tam Ekran `/watch` Oynatıcı Özellikleri

- **Çok Platformlu Oynatıcı (Masaüstü & Android):** Masaüstünde WebKit `/watch` ve harici **`mpv`** aktarımı (`vessel-player-mpv`); Android'de varsayılan olarak yatay (**landscape**) açılan **Media3 ExoPlayer** arayüzü ve kompakt kontrol çekmeceleri. *(Not: Canlı TV eklenti kanalları içerik detay sayfasına uğramadan doğrudan oynatıcıyı açar; canlı yayınlarda içerik linki, ses parçası seçimi ve indirme butonu gizlenir.)*
- **OynatmaMotorları:** HLS (`.m3u8`), MPEG-TS, MP4/WebM ve yerel çevrimdışı yayınlar.
- **İçerik Sayfasına Doğrudan Dönüş:** Oynatılan videodan ayrılmadan üst bardaki **Detaylar** butonuna basarak doğrudan o içeriğin detay sayfasına dönebilirsiniz.
- **Ses Parçaları Seçimi:** HLS master manifestlerinden veya yerel video öğesinden gelen çoklu ses kanallarını oynatıcı içindeki **Ses** çekmecesinden seçebilirsiniz.
- **Gelişmiş Altyazılar & Önizleme:** `.srt`, `.vtt`, `.ass`, `.ssa` desteği; oynatıcı içi `-500ms / +500ms` gecikme ayarı; **Ayarlar → Altyazı Özelleştirme** altında yazı tipi, boyut, kenar stili (`Outline`, `Shadow`, `Box`, `Clean`), arka plan opaklığı, alt boşluk ve **canlı altyazı önizleme kutusu**.
- **Çekmeceler & Ekran Kilidi:** Videodan çıkmadan **Bölümler** (küçük resim + özet), **Kaynaklar** ve **Ekran Kilidi** erişimi.

### Klavye Kısayolları & Touchpad Kontrolleri

| Kısayol / Hareket | İşlev |
| :--- | :--- |
| `Space` / `K` | Oynat / Duraklat |
| `←` / `J` ve `→` / `L` | 10 sn geri / ileri sar |
| `↑` / `↓` | Ses artır / azalt (`%5`) |
| `M` / `F` / `Esc` | Sessize al / Tam ekran / Kapat-Geri dön |
| **İki parmak yatay kaydırma** | Akıcı ileri/geri sarma |
| **İki parmak dikey kaydırma** | Ses seviyesi |
| **İki parmak kıstırma (`Ctrl + Scroll`)** | Ekranı doldur / sığdır (`cover` / `contain`) |

> **Doğal Touchpad Kaydırma:** macOS veya trackpad'inizde yönler ters geliyorsa **Ayarlar → Oynatma ve Gizlilik → Doğal Touchpad Kaydırma** seçeneğini açın.

---

## 2. Akıllı Bölüm Atlama (`Skip Intro / Recap / Credits / Preview`)

Video oynatılırken kaynak veya topluluk veritabanlarından alınan atlama aralıklarında ekran köşesinde tek tıkla **Atla** butonu çıkar (istenirse o segment kapatılabilir):

- **Öncelik Sırası:** **`Eklenti/Akış Verisi > TheIntroDB > IntroDB > AniSkip`**
- Eksik `mal_id` / `imdb_id` / `tmdb_id` kimlikleri varsa `AniList` ID üzerinden `Ani.zip` ile eşleştirilir.
- **Hatalı Atlama Korumaları:**
  - `AniSkip` yalnızca `sezon <= 1` için çalışır ve video süresi ile veritabanındaki bölüm süresi farkı `120 sn`'den büyükse segment reddedilir.
  - `IntroDB` topluluk tarafından `no_intro` / `no_recap` / `no_outro` olarak işaretlenmişse `AniSkip` üzerine yazmaz ve `0.40` altındaki güven skorlarını eler.
  - `3 sn` altındaki veya toplam sürenin `%85`'ini aşan mantıksız aralıklar otomatik filtrelenir.
  - **Gizlilik:** Ayarlarda harici metadata kapalıysa (`metadataEnabled = false`) dış atlama veritabanlarına hiçbir sorgu atılmaz.

---

## 3. Çevrimdışı İndirmeler & 4 Seviyeli İndirme Kütüphanesi

### İndirme Motoru (`vessel-downloader`)
- **Eşzamanlı Transfer:** Direct videolarda HTTP `Accept-Ranges` parçalı çoklu worker, HLS yayınlarda ise **8 paralel worker** ile AES-128 şifre çözme.
- **Bağlantılı Dosyalar:** Bir bölüm indirilirken varsa akışa ait harici altyazılar ve HLS master manifestindeki ses/altyazı kanalları da paralel olarak çevrimdışı klasöre indirilir.
- **Anında Duraklatma ve Silme:** `tokio::select! { biased; ... }` + `join_set.abort_all()` sayesinde duraklatma veya silme anında açık HTTP isteklerini keser, yarım klasörleri temizler ve sıradaki indirmeyi başlatır.
- **Sürükle-Bırak Kuyruk Sırası:** İndirme Kuyruğu listesindeki tutamaçları sürükleyerek öncelik sırasını değiştirebilirsiniz.
- **Özel Konum:** İndirmelerin nereye kaydedileceğini **Ayarlar → İndirme Konumu** bölümünden seçebilirsiniz.

### 4 Seviyeli İndirme Ağacı
**İndirmeler** ekranı dosyaları düz liste yerine hiyerarşik olarak gruplar:
- **Diziler / Animeler:** **İçerik → Sezon → Bölüm → İndirilen Kaynak (`streamLabel`)**
  *(Aynı bölüm için 2 farklı kaynak indirdiyseniz ikisi de o bölümün altında ayrı tıklanabilir satırlar halinde listelenir.)*
- **Filmler:** Doğrudan **İçerik → İndirilen Kaynak (`streamLabel`)**
- Her seviyede izlenme durumu (`İzlendi` yeşil rozeti), bölüm ilerleme çubuğu ve çevrimdışı **İzlemeye Devam Et** şeridi bulunur.
