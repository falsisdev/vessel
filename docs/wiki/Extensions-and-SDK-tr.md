> 🌐 [English (Extensions)](Extensions-and-Repositories) · [English (Plugin SDK)](Plugin-Development-SDK) · **Türkçe** · [Türkçe Ana Sayfa](Home-tr)

# Eklentiler, Canlı TV (IPTV) ve Geliştirici SDK

---

## 1. Eklenti & Repo Kurulumu

**Eklentiler → Kaynak İncele** kutusuna aşağıdaki biçimlerden herhangi birini yapıştırabilirsiniz:
- Doğrudan HTTPS bağlantısı (`https://.../repo.json` veya `.plugin.vessel`)
- GitHub / GitLab kısa yolu (`gh:kullanici/repo`, `gl:grup/proje@dal/altklasor`)
- Kayıt defteri kısa kodu (`registry.json` veya `301/302` kısa URL — örn. `anthology` veya `tr` → `gh:falsisdev/anthology`)
- Deep link (`vessel://install?src=...`)
- Yerel **Dev Mode** klasör yolu (`/Users/.../benim-eklentim`)
- Açık kaynak **Nuvio** kazıyıcı manifestleri

### Alan Adı Koruması & İzinler
Eklentiler internete yalnızca `manifest.json` içinde beyan ettikleri alan adlarından veya kullanıcının çalışma anında **Bir Kez İzin Ver / Her Zaman İzin Ver** dediği adreslerden çıkabilir. Yerel ağ IP adreslerine (`127.0.0.1`, `192.168.x.x`, `10.x.x.x`) yapılan istekler DNS çözümlemesinden sonra bile **SSRF korumasıyla** kesin olarak engellenir.

---

## 2. Canlı TV, IPTV ve XMLTV EPG

**Canlı TV** ekranında iki sekme yer alır:
1. **M3U / Xtream:** M3U+ oynatma listesi veya Xtream Codes (`Sunucu + Kullanıcı Adı + Şifre`) profili ekleyin; kanal özel `User-Agent` tanımlayın; XMLTV EPG ile kanallardaki **Şimdi** ve **Sonraki** yayın bilgilerini görüntüleyin; SQLite **FTS5** ile anında arama yapın ve favorilerinizi kaydedin.
2. **Canlı TV Eklentileri:** `live_tv` yeteneğine sahip kurulu eklentilerden gelen canlı HLS/MPEG-TS kanallarını tek yerden filtreleyip izleyin.

---

## 3. Hızlı Eklenti Geliştirme Özeti (TypeScript)

Eklentiler QuickJS sandbox'ında (`32 MiB` heap, `512 KiB` stack, `15 sn` zaman aşımı) çalışır ve `oxc` derleyicisi TypeScript tiplerini anında soyup SQLite önbelleğine alır. Node.js veya harici paketleyici gerekmez.

```typescript
export default vessel.definePlugin({
  async getHomeFeed(cursor) {
    const res = await vessel.http.get("https://example.org/");
    const doc = vessel.html.parse(res.text());
    return { sections: [], nextCursor: null };
  },
  async search(query, filters, page) {
    return { items: [], hasNextPage: false };
  },
  async getDetails(nativeId) {
    return { nativeId, type: "series", isAnime: false, title: "Demo", episodes: [] };
  },
  async getStreams(streamId) {
    return { streams: [], subtitles: [] };
  }
});
```

- **Yerleşik Köprüler:** `vessel.http`, `vessel.html.parse`, `vessel.crypto`, `vessel.storage`, `vessel.unpack`, `vessel.evalWithEnv`
- **Uyumluluk Katmanı:** `fetch`, `URL`, `URLSearchParams`, `TextEncoder`/`TextDecoder`, `atob`/`btoa`, `require("cheerio" | "crypto-js" | "axios" | "path")`
- Detaylı şema, tüm TypeScript alanları ve hata standartları (`[Vessel<Variant>Error]`) için tam İngilizce **[Plugin Development SDK](Plugin-Development-SDK)** sayfasına göz atın.
