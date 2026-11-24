> 🌐 [English](Desktop-App-User-Guide) · **Türkçe** · [Türkçe Ana Sayfa](Home-tr)

# Masaüstü Uygulaması Kullanım Rehberi

Bu rehber **Ana Sayfa**, **Arama**, **İçerik Detayı**, **Kütüphane** ve **Ayarlar** ekranlarındaki tüm özellikleri kapsar.

---

## 1. Ana Sayfa (`HomeView`)

### Vitrin Hero Alanı & Kaynak Seçimi
- Otomatik geçiş yapan tam ekran vitrin bandı; içerik yılı, puanı, özeti, **Hemen İzle** ve **Detaylar** butonlarını sunar.
- **Ana Sayfa Akışlarını Seç (`Feeds`)** penceresinden hangi eklentilerin Ana Sayfa raflarında ve Hero alanında görüneceğini seçebilir, rafları sürükleyip bırakarak sıralayabilirsiniz. Ana sayfada kapatılan eklentiler **Arama** ekranında aktif kalmaya devam eder.

### Akıllı "İzlemeye Devam Et"
- **Filmler:** Sezon/bölüm etiketi gösterilmez; film tamamlandığında (`%90` eşiği sonrası) İzlemeye Devam Et şeridinden otomatik kaldırılır, ancak veritabanında izlendi olarak korunur.
- **Diziler & Animeler:** Bir bölüm bitirildiğinde o bölüm kaldırılır ve varsa **sonraki bölüm** (`Sıradaki`) otomatik olarak İzlemeye Devam Et şeridine taşınır. Dizinin son bölümü bitirildiğinde dizi listeden kalkar.
- **Tek Tip Kart Boyutu & Kısaltma:** Tüm kartlar eşit boyuttadır ve uzun bölüm başlıkları kart düzenini bozmayacak şekilde `...` ile kısaltılır.
- **Sağ Tık / Uzun Basma Menüsü:** Herhangi bir kartın üzerine sağ tıklayarak (veya basılı tutarak) doğrudan **İçeriğe Git (`Go to Details`)** seçeneğine ulaşabilirsiniz.
- **Manuel Düzenleme Penceresi (Kalem Simgesi):** İzlemeye Devam Et kartındaki kalem simgesine basarak sezon/bölüm koordinatını ve izlenen dakikayı el ile düzenleyebilirsiniz. **Toplam Süre** alanı değiştirilemez; seçilen bölümün gerçek süresine göre otomatik güncellenir.

### Hızlı İzleme Durumu Butonu
Ana Sayfa ve Arama ekranındaki tüm içerik kartlarının sağ üst köşesindeki hızlı durum simgesinden içerik sayfasına girmeden **İzleniyor**, **İzlenecek**, **Tamamlandı**, **Beklemede** veya **Bırakıldı** olarak işaretleyebilirsiniz.

---

## 2. Arama (`Cmd/Ctrl + K`)

- Tüm etkin eklentilerde **paralel canlı akışlı arama** yapar; sonuçlar eklentilerden geldikçe ekrana düşer.
- **Tür Filtresi:** `Tümü`, `Filmler`, `Diziler`, `Anime`.
- **Tek Eklenti Filtresi:** Yalnızca seçtiğiniz tek bir eklenti içinde arama yapabilirsiniz.

---

## 3. İçerik Detayı (`DetailsView`)

- **Bağlamsal İzle / Devam Et Butonu:** İçerikte kayıtlı ilerlemeniz varsa ana buton otomatik olarak **Devam Et** etiketini alır.
- **Eşleşmeyi Düzelt (`Fix Match`):** Harici metadata eşleşmesi yanlışsa `anilist:<id>`, `tmdb:tv:<id>` veya `imdb:tt<id>` yazarak anında düzeltebilir ya da harici metadatayı o içerik için tamamen kapatabilirsiniz.
- **Gerçek Bölüm Başlıkları & İlerleme:** Bölüm kartlarında `1. Sezon 3. Bölüm` gibi jenerik metinler yerine varsa metadatadaki gerçek bölüm başlığı gösterilir. Bölüm kartlarının altında izleme ilerleme çubuğu, biten bölümlerin yanında ise **İzlendi** simgesi görünür.
- **3 Bölüm Görünüm Modu:** `Kart Izgara`, `Detaylı Liste`, `Kompakt Satırlar`.
- **Kaynak Seçici & İndirme:** Bölüm veya akış seçildiğinde sunucu kalitesini, ses türünü (`Sub` / `Dub` / `Original`) seçebilir, doğrudan oynatabilir veya çevrimdışı indirmeyi başlatabilirsiniz.

---

## 4. Kütüphane (`LibraryView`)

- Tüm izleme durumlarına göre filtreleme yapabilirsiniz.
- Kartların **sol üst köşesindeki kaldırma (`×`) butonu**, sağ üstteki hızlı izleme durumu simgesiyle çakışmadan tek tıkla silme sağlar.

---

## 5. Ayarlar, Temalar, 12 Dil ve Yedekleme

- **5 Yerleşik Tema + Özel `.theme.vessel` İçe Aktarma:** `Catppuccin Mocha`, `Catppuccin Latte`, `OLED Pitch Black`, `Nord`, `Tokyo Night` ve özel CSS desteği.
- **12 Yerleşik Dil (Kendi Adlarıyla):** Türkçe, İngilizce, İspanyolca, Almanca, Fransızca, İtalyanca, Rusça, Japonca, Portekizce, Çince, Arapça, Farsça.
- **Görünüm Düzeni:** `Yüzen Alt Dock (Floating Bottom)`, `Geniş Kenar Çubuğu`, `Sabit Kenar Çubuğu`, `Kompakt Şerit`, `Üst Menü`, `Gizli`.
- **Taşınabilir Yedekler (`.backup.vessel`):** `Akıllı Birleştirme` (yeni izleme zamanlarını korur) veya `Sıfırla ve Değiştir` modlarıyla tüm arşivinizi dışa/içe aktarabilirsiniz.
