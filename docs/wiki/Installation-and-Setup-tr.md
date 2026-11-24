> 🌐 [English](Installation-and-Setup) · **Türkçe** · [Türkçe Ana Sayfa](Home-tr)

# Kurulum, Veri Dizini ve Kaynaktan Derleme

Bu sayfa **Vessel** masaüstü uygulamasının **macOS** ve **Windows** kurulum adımlarını, yerel veri dizinini (`<VesselRoot>`) ve kaynak koddan derleme komutlarını içerir.

---

## 1. Hazır Sürümleri İndirme (`GitHub Releases`)

Etiketlenen her sürüm (`v*`) GitHub Actions üzerinden otomatik derlenerek **[GitHub Releases](https://github.com/falsisdev/vessel/releases)** sayfasında paylaşılır:

| İşletim Sistemi | Mimari | Paket Formatı | Açıklama |
| :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon (`aarch64-apple-darwin`) | `.dmg` | `Vessel.app` dosyasını Applications klasörüne sürükleyin |
| **macOS** | Intel (`x86_64-apple-darwin`) | `.dmg` | `Vessel.app` dosyasını Applications klasörüne sürükleyin |
| **Windows** | x64 (`x86_64-pc-windows-msvc`) | `-setup.exe` (NSIS) | Kurulum sihirbazı |
| **Linux** | x64 (`x86_64-unknown-linux-gnu`) | `.AppImage`, `.deb`, `.rpm`, `.flatpak` | Flatpak sandbox (`dev.falsis.vessel`) + yerel paketler |

---

### Linux Kurulum Notları (`.AppImage`, `.deb`, `.rpm`, `.flatpak`)

- **AppImage:** `chmod +x Vessel_*.AppImage && ./Vessel_*.AppImage`
- **Debian / Ubuntu (`.deb`):** `sudo apt install ./Vessel_*.deb`
- **Fedora / RHEL (`.rpm`):** `sudo dnf install ./Vessel-*.rpm`
- **Flatpak (`.flatpak`):**
  ```bash
  flatpak install --user Vessel_*_amd64.flatpak
  flatpak run dev.falsis.vessel
  ```
  > Flatpak içinde `<VesselRoot>` otomatik olarak XDG uygulama veri dizinine (`~/.var/app/dev.falsis.vessel/data/dev.falsis.vessel`) yerleşir.

---

### macOS Notu (`"Vessel bozuk ve açılamıyor"` Uyarısı)

Vessel açık kaynaklı ve ücretli Apple Developer ID sertifikası olmadan imzalandığı için macOS Gatekeeper internetten indirilen `.dmg` paketlerine karantina etiketi (`com.apple.quarantine`) ekler.

1. `.dmg` dosyasını açıp **Vessel.app** uygulamasını **Uygulamalar (`Applications`)** klasörüne sürükleyin.
2. **Terminal**'i açıp şu komutu bir kez çalıştırın:
   ```bash
   xattr -cr /Applications/Vessel.app
   ```
3. Ardından **Vessel**'i normal şekilde açabilirsiniz. *(Yeni bir güncelleme yüklediğinizde gerekirse aynı komutu tekrar çalıştırın.)*

---

### Windows Notu (SmartScreen)

İmzasız alpha/beta kurulum dosyalarında Windows SmartScreen *"Windows kişisel bilgisayarınızı korudu"* uyarısı gösterirse **Ek bilgi → Yine de çalıştır** seçeneğine tıklayın.

---

### Opsiyonel: Harici `mpv` Oynatıcı

Uygulama içi WebView oynatıcı HLS, MP4, WebM ve MPEG-TS akışlarını doğrudan oynatır. **MKV**, **HEVC/H.265** veya özel ses kodeklerini harici **`mpv`** ile açmak için sisteminize `mpv` kurabilirsiniz:
- **macOS:** `brew install mpv`
- **Windows:** `winget install mpv.net` veya `scoop install mpv`
- **Linux:** `sudo apt install mpv` veya `sudo pacman -S mpv`

---

## 2. Yerel Veri Klasörü (`<VesselRoot>`)

Tüm veritabanınız, eklentileriniz ve indirmeleriniz yalnızca kendi diskinizde saklanır:

```text
<VesselRoot>/
├── storage.db           # SQLite (WAL + FTS5): kütüphane, izleme geçmişi, eklentiler, alan izinleri, IPTV, önbellek
├── downloads/           # Varsayılan çevrimdışı indirme klasörü (Ayarlar -> İndirme Konumu üzerinden değiştirilebilir)
├── plugins/             # Kurulu .plugin.vessel eklenti paketleri
├── themes/              # İçe aktarılan *.theme.vessel JSON renk temaları
├── locales/             # Ekstra <dil>.json arayüz çevirileri
└── backups/             # Dışa aktarılan *.backup.vessel yedek dosyaları
```

---

## 3. Kaynaktan Derleme ve Test

### Gereksinimler
- **Rust** (`1.89+`, `rustup default stable`)
- **Node.js** (`v20+`, CI'da `v22`) ve **pnpm** (`v10+`)
- **Tauri v2 Sistem Bağımlılıkları** (macOS'ta Xcode Command Line Tools; Windows'ta C++ Build Tools + WebView2)

### Komutlar
```bash
git clone https://github.com/falsisdev/vessel.git
cd vessel

pnpm install

# Geliştirme modunda masaüstü uygulamasını başlat
pnpm --filter @vessel/desktop tauri dev

# Tüm Rust testlerini ve Clippy kontrollerini çalıştır
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Vue/TypeScript kontrolü ve üretim derlemesi
pnpm --filter @vessel/desktop build
pnpm --filter @vessel/desktop tauri build
```
