/// Built-in multi-provider subtitle container for Vessel.

declare const vessel: {
  definePlugin<T>(plugin: T): T;
  http: {
    get(url: string, opts?: Record<string, unknown>): Promise<{
      status: number;
      url: string;
      headers: Record<string, string>;
      text(encoding?: string): string;
      json<T = unknown>(): T;
      bytes(): Uint8Array;
    }>;
    post(url: string, body?: unknown, opts?: Record<string, unknown>): Promise<{
      status: number;
      url: string;
      headers: Record<string, string>;
      text(encoding?: string): string;
      json<T = unknown>(): T;
      bytes(): Uint8Array;
    }>;
  };
  html: {
    parse(markup: string): {
      select(selector: string): Array<{
        text(): string;
        html(): string;
        attr(name: string): string | null;
        selectOne(selector: string): {
          text(): string;
          html(): string;
          attr(name: string): string | null;
        } | null;
        select(selector: string): Array<{
          text(): string;
          html(): string;
          attr(name: string): string | null;
        }>;
      }>;
      selectOne(selector: string): {
        text(): string;
        html(): string;
        attr(name: string): string | null;
      } | null;
    };
  };
  storage: {
    getSetting(key: string): Promise<string | boolean | null>;
  };
  archive: {
    unzipEntries(bytes: Uint8Array): Array<{
      name: string;
      bytes: Uint8Array;
      text(encoding?: string): string;
    }>;
    gunzip(bytes: Uint8Array): Uint8Array;
    decodeText(bytes: Uint8Array, encoding?: string): string;
  };
};

type SubtitleFormat = "srt" | "ass" | "ssa" | "vtt";

interface SubtitleQuery {
  title: string;
  originalTitle?: string;
  year?: number;
  mediaType: "movie" | "series" | "live";
  season?: number;
  episode?: number;
  externalIds?: {
    tmdb?: number;
    imdb?: string;
  };
  preferredLanguages?: string[];
}

interface SubtitleCandidate {
  id: string;
  lang: string;
  label: string;
  format: SubtitleFormat;
  releaseName?: string;
  sourceName?: string;
  downloads?: number;
  hearingImpaired?: boolean;
}

interface ResolvedSubtitle {
  format: SubtitleFormat;
  source:
    | { kind: "url"; url: string; headers?: Record<string, string> }
    | { kind: "inline_content"; content: string };
}

interface ResolveToken {
  p: string;
  u: string;
  f?: SubtitleFormat;
  r?: string;
  s?: number;
  e?: number;
  form?: Record<string, string>;
  k?: string;
  fileId?: number;
}

const UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

const LANG_NAME_TO_CODE: Record<string, string> = {
  turkish: "tr",
  tur: "tr",
  "türkçe": "tr",
  english: "en",
  eng: "en",
  spanish: "es",
  spa: "es",
  french: "fr",
  fre: "fr",
  fra: "fr",
  german: "de",
  ger: "de",
  deu: "de",
  italian: "it",
  ita: "it",
  portuguese: "pt",
  por: "pt",
  "brazilian portuguese": "pt-br",
  brazilian: "pt-br",
  russian: "ru",
  rus: "ru",
  japanese: "ja",
  jpn: "ja",
  arabic: "ar",
  ara: "ar",
  persian: "fa",
  farsi: "fa",
  per: "fa",
  chinese: "zh",
  chi: "zh",
  zho: "zh",
  korean: "ko",
  kor: "ko",
  dutch: "nl",
  polish: "pl",
  romanian: "ro",
  greek: "el",
  ukrainian: "uk",
  indonesian: "id",
  vietnamese: "vi",
  hindi: "hi",
  swedish: "sv",
  norwegian: "no",
  danish: "da",
  finnish: "fi",
  czech: "cs",
  hungarian: "hu",
};

const CODE_TO_3: Record<string, string> = {
  tr: "tur",
  en: "eng",
  es: "spa",
  fr: "fre",
  de: "ger",
  it: "ita",
  pt: "por",
  ru: "rus",
  ja: "jpn",
  ar: "ara",
  fa: "per",
  zh: "chi",
  ko: "kor",
  nl: "dut",
  pl: "pol",
  ro: "rum",
  el: "ell",
  uk: "ukr",
  id: "ind",
  vi: "vie",
  hi: "hin",
};

async function readBoolSetting(key: string, defaultVal: boolean): Promise<boolean> {
  const val = await vessel.storage.getSetting(key);
  if (typeof val === "boolean") return val;
  if (typeof val === "string") {
    const low = val.trim().toLowerCase();
    if (low === "false" || low === "0") return false;
    if (low === "true" || low === "1") return true;
  }
  return defaultVal;
}

async function readStrSetting(key: string): Promise<string> {
  const val = await vessel.storage.getSetting(key);
  return typeof val === "string" ? val.trim() : "";
}

function normalizeLang(raw: string): string {
  const clean = raw.trim().toLowerCase().replace(/_/g, "-");
  if (!clean) return "und";
  if (LANG_NAME_TO_CODE[clean]) return LANG_NAME_TO_CODE[clean];
  const base = clean.split("-")[0];
  if (LANG_NAME_TO_CODE[base]) return LANG_NAME_TO_CODE[base];
  if (clean.length === 2) return clean;
  return clean.slice(0, 5);
}

function langDisplayName(code: string): string {
  const map: Record<string, string> = {
    tr: "Türkçe",
    en: "English",
    es: "Español",
    fr: "Français",
    de: "Deutsch",
    it: "Italiano",
    pt: "Português",
    "pt-br": "Português (BR)",
    ru: "Русский",
    ja: "日本語",
    ar: "العربية",
    fa: "فارسی",
    zh: "中文",
    ko: "한국어",
  };
  return map[code] || code.toUpperCase();
}

function detectFormat(nameOrUrl: string, fallback: SubtitleFormat = "srt"): SubtitleFormat {
  const lower = nameOrUrl.toLowerCase().split("?")[0];
  if (lower.endsWith(".ass")) return "ass";
  if (lower.endsWith(".ssa")) return "ssa";
  if (lower.endsWith(".vtt")) return "vtt";
  if (lower.endsWith(".srt")) return "srt";
  return fallback;
}

function isSupportedSubFile(name: string): boolean {
  const lower = name.toLowerCase();
  return (
    !lower.startsWith("__macosx/") &&
    (lower.endsWith(".srt") ||
      lower.endsWith(".ass") ||
      lower.endsWith(".ssa") ||
      lower.endsWith(".vtt"))
  );
}

function normalizeText(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

function episodeRegex(season?: number, episode?: number): RegExp | null {
  if (!season || !episode) return null;
  const s = String(season);
  const s2 = pad2(season);
  const e = String(episode);
  const e2 = pad2(episode);
  return new RegExp(
    `(?:s0*${s}[\\s._-]*e0*${e}\\b|\\b${s}x0*${e}\\b|\\b0*${s2}x0*${e2}\\b|\\bep\\.?\\s*0*${e}\\b)`,
    "i",
  );
}

function pickBestZipEntry(
  entries: Array<{ name: string; bytes: Uint8Array; text(encoding?: string): string }>,
  season?: number,
  episode?: number,
): { format: SubtitleFormat; content: string } {
  const subs = entries.filter((e) => isSupportedSubFile(e.name));
  if (!subs.length) {
    throw new Error("ZIP archive does not contain a supported subtitle (.srt/.ass/.ssa/.vtt)");
  }

  let chosen = subs[0];
  const epRx = episodeRegex(season, episode);
  if (epRx && subs.length > 1) {
    const matched = subs.find((e) => epRx.test(e.name));
    if (matched) chosen = matched;
  } else if (subs.length > 1) {
    const nonHi = subs.find((e) => !/\b(sdh|hi|forced|commentary)\b/i.test(e.name));
    if (nonHi) chosen = nonHi;
  }

  return {
    format: detectFormat(chosen.name, "srt"),
    content: chosen.text("auto"),
  };
}

function encodeToken(tok: ResolveToken): string {
  return JSON.stringify(tok);
}

function decodeToken(raw: string): ResolveToken {
  return JSON.parse(raw) as ResolveToken;
}

function normalizeImdb(imdb?: string): string | null {
  if (!imdb) return null;
  const trimmed = imdb.trim();
  if (/^tt\d+$/.test(trimmed)) return trimmed;
  if (/^\d+$/.test(trimmed)) return `tt${trimmed}`;
  return null;
}

function preferredSet(query: SubtitleQuery): string[] {
  const raw = query.preferredLanguages ?? [];
  const out: string[] = [];
  for (const lang of raw) {
    const code = normalizeLang(lang);
    if (code && code !== "und" && !out.includes(code)) out.push(code);
  }
  return out;
}

function matchesPreferred(langCode: string, prefs: string[]): boolean {
  if (!prefs.length) return true;
  const base = langCode.split("-")[0];
  return prefs.includes(langCode) || prefs.includes(base);
}

// ---------------------------------------------------------------------------
// 1. OpenSubtitles (Official REST API v1 when apiKey set; Stremio v3 fallback)
// ---------------------------------------------------------------------------
async function searchOpenSubtitles(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("openSubtitlesApiKey");
  const imdb = normalizeImdb(query.externalIds?.imdb);
  const tmdb = query.externalIds?.tmdb;

  if (apiKey) {
    const params = new URLSearchParams();
    if (imdb) {
      params.set("imdb_id", imdb.replace(/^tt/, ""));
    } else if (tmdb) {
      params.set("tmdb_id", String(tmdb));
    }
    const qText = (query.originalTitle || query.title || "").trim();
    if (qText && !imdb && !tmdb) params.set("query", qText);
    if (query.mediaType === "movie") params.set("type", "movie");
    if (query.mediaType === "series") {
      params.set("type", "episode");
      if (query.season) params.set("season_number", String(query.season));
      if (query.episode) params.set("episode_number", String(query.episode));
    }
    if (prefs.length) {
      params.set("languages", prefs.join(","));
    }

    const resp = await vessel.http.get(
      `https://api.opensubtitles.com/api/v1/subtitles?${params.toString()}`,
      {
        headers: {
          "Api-Key": apiKey,
          "User-Agent": "Vessel v0.2.0",
          Accept: "application/json",
        },
        timeoutMs: 12000,
      },
    );
    if (resp.status >= 400) {
      throw new Error(`OpenSubtitles API HTTP ${resp.status}`);
    }
    const data = resp.json<{
      data?: Array<{
        attributes?: {
          language?: string;
          release?: string;
          download_count?: number;
          hearing_impaired?: boolean;
          format?: string;
          files?: Array<{ file_id?: number; file_name?: string }>;
        };
      }>;
    }>();

    const out: SubtitleCandidate[] = [];
    for (const item of data.data ?? []) {
      const attrs = item.attributes;
      const file = attrs?.files?.[0];
      if (!attrs || !file?.file_id) continue;
      const lang = normalizeLang(attrs.language || "en");
      const fmt = detectFormat(file.file_name || `.${attrs.format || "srt"}`, "srt");
      const rel = (attrs.release || file.file_name || "").trim();
      out.push({
        id: encodeToken({ p: "os_api", u: "https://api.opensubtitles.com/api/v1/download", fileId: file.file_id, f: fmt, k: apiKey }),
        lang,
        label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
        format: fmt,
        releaseName: rel || undefined,
        sourceName: "OpenSubtitles",
        downloads: attrs.download_count,
        hearingImpaired: Boolean(attrs.hearing_impaired),
      });
    }
    return out;
  }

  // API key-free Stremio OpenSubtitles v3 endpoint (requires IMDb ID).
  if (!imdb) return [];
  const isSeries = query.mediaType === "series" && query.season && query.episode;
  const stremioType = isSeries ? "series" : "movie";
  const stremioId = isSeries ? `${imdb}:${query.season}:${query.episode}` : imdb;
  const resp = await vessel.http.get(
    `https://opensubtitles-v3.strem.io/subtitles/${stremioType}/${encodeURIComponent(stremioId)}.json`,
    { headers: { "User-Agent": UA }, timeoutMs: 12000 },
  );
  if (resp.status >= 400) {
    throw new Error(`OpenSubtitles v3 HTTP ${resp.status}`);
  }
  const json = resp.json<{
    subtitles?: Array<{
      id?: string;
      url?: string;
      lang?: string;
      SubEncoding?: string;
    }>;
  }>();

  const out: SubtitleCandidate[] = [];
  for (const sub of json.subtitles ?? []) {
    if (!sub.url) continue;
    const lang = normalizeLang(sub.lang || "en");
    if (!matchesPreferred(lang, prefs)) continue;
    const fmt = detectFormat(sub.url, "srt");
    out.push({
      id: encodeToken({ p: "direct", u: sub.url, f: fmt }),
      lang,
      label: `${langDisplayName(lang)} · OpenSubtitles v3`,
      format: fmt,
      sourceName: "OpenSubtitles",
    });
  }
  return out.slice(0, 40);
}

async function resolveOpenSubtitlesApi(tok: ResolveToken): Promise<ResolvedSubtitle> {
  const apiKey = tok.k || (await readStrSetting("openSubtitlesApiKey"));
  const username = await readStrSetting("openSubtitlesUsername");
  const password = await readStrSetting("openSubtitlesPassword");
  const headers: Record<string, string> = {
    "Api-Key": apiKey,
    "User-Agent": "Vessel v0.2.0",
    "Content-Type": "application/json",
    Accept: "application/json",
  };

  if (username && password) {
    const loginResp = await vessel.http.post(
      "https://api.opensubtitles.com/api/v1/login",
      { username, password },
      { headers, timeoutMs: 10000 },
    );
    if (loginResp.status < 400) {
      const loginJson = loginResp.json<{ token?: string }>();
      if (loginJson.token) headers["Authorization"] = `Bearer ${loginJson.token}`;
    }
  }

  const dlResp = await vessel.http.post(
    "https://api.opensubtitles.com/api/v1/download",
    { file_id: tok.fileId },
    { headers, timeoutMs: 12000 },
  );
  if (dlResp.status >= 400) {
    throw new Error(`OpenSubtitles download HTTP ${dlResp.status}`);
  }
  const dlJson = dlResp.json<{ link?: string }>();
  if (!dlJson.link) {
    throw new Error("OpenSubtitles download response missing link");
  }
  return {
    format: tok.f || "srt",
    source: { kind: "url", url: dlJson.link },
  };
}

// ---------------------------------------------------------------------------
// 2. SubDL (API key required)
// ---------------------------------------------------------------------------
async function searchSubdl(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("subdlApiKey");
  if (!apiKey) {
    return [];
  }

  const params = new URLSearchParams();
  params.set("api_key", apiKey);
  const imdb = normalizeImdb(query.externalIds?.imdb);
  if (imdb) {
    params.set("imdb_id", imdb);
  } else if (query.externalIds?.tmdb) {
    params.set("tmdb_id", String(query.externalIds.tmdb));
  } else {
    params.set("film_name", query.originalTitle || query.title);
    if (query.year) params.set("year", String(query.year));
  }
  params.set("type", query.mediaType === "series" ? "tv" : "movie");
  if (query.mediaType === "series") {
    if (query.season) params.set("season_number", String(query.season));
    if (query.episode) params.set("episode_number", String(query.episode));
  }
  if (prefs.length) {
    params.set("languages", prefs.map((c) => c.toUpperCase()).join(","));
  }
  params.set("subs_per_page", "30");

  const resp = await vessel.http.get(`https://api.subdl.com/api/v1/subtitles?${params.toString()}`, {
    headers: { "User-Agent": UA, Accept: "application/json" },
    timeoutMs: 12000,
  });
  if (resp.status >= 400) {
    throw new Error(`SubDL HTTP ${resp.status}`);
  }
  const data = resp.json<{
    status?: boolean;
    error?: string;
    subtitles?: Array<{
      release_name?: string;
      name?: string;
      lang?: string;
      url?: string;
      hi?: boolean;
    }>;
  }>();
  if (data.status === false && data.error) {
    throw new Error(`SubDL: ${data.error}`);
  }

  const out: SubtitleCandidate[] = [];
  for (const item of data.subtitles ?? []) {
    if (!item.url) continue;
    const lang = normalizeLang(item.lang || "en");
    const dlUrl = item.url.startsWith("http")
      ? item.url
      : `https://dl.subdl.com${item.url.startsWith("/") ? "" : "/"}${item.url}`;
    const rel = (item.release_name || item.name || "").trim();
    out.push({
      id: encodeToken({
        p: "zip",
        u: dlUrl,
        s: query.season,
        e: query.episode,
      }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
      format: "srt",
      releaseName: rel || undefined,
      sourceName: "SubDL",
      hearingImpaired: Boolean(item.hi),
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// 3. SubSource (API key required)
// ---------------------------------------------------------------------------
async function searchSubsource(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("subsourceApiKey");
  if (!apiKey) {
    return [];
  }
  const headers = {
    "X-API-Key": apiKey,
    "User-Agent": UA,
    Accept: "application/json",
  };
  const imdb = normalizeImdb(query.externalIds?.imdb);
  const searchParams = new URLSearchParams();
  if (imdb) {
    searchParams.set("searchType", "imdb");
    searchParams.set("imdb", imdb);
  } else {
    searchParams.set("searchType", "text");
    searchParams.set("q", query.originalTitle || query.title);
    if (query.year) searchParams.set("year", String(query.year));
  }

  const movieResp = await vessel.http.get(
    `https://api.subsource.net/api/v1/movies/search?${searchParams.toString()}`,
    { headers, timeoutMs: 12000 },
  );
  if (movieResp.status >= 400) {
    throw new Error(`SubSource HTTP ${movieResp.status}`);
  }
  const movieJson = movieResp.json<{
    data?: Array<{ movieId?: number; id?: number; title?: string }>;
  }>();
  const movieId = movieJson.data?.[0]?.movieId ?? movieJson.data?.[0]?.id;
  if (!movieId) return [];

  const subParams = new URLSearchParams({ movieId: String(movieId), limit: "30" });
  if (prefs.length) subParams.set("language", prefs.join(","));
  const subResp = await vessel.http.get(
    `https://api.subsource.net/api/v1/subtitles?${subParams.toString()}`,
    { headers, timeoutMs: 12000 },
  );
  if (subResp.status >= 400) {
    throw new Error(`SubSource subtitles HTTP ${subResp.status}`);
  }
  const subJson = subResp.json<{
    data?: Array<{
      subtitleId?: number;
      id?: number;
      language?: string;
      releaseInfo?: string[];
      downloads?: number;
      hearingImpaired?: boolean;
    }>;
  }>();

  const out: SubtitleCandidate[] = [];
  for (const item of subJson.data ?? []) {
    const sid = item.subtitleId ?? item.id;
    if (!sid) continue;
    const lang = normalizeLang(item.language || "en");
    if (!matchesPreferred(lang, prefs)) continue;
    const rel = (item.releaseInfo ?? []).join(" ").trim();
    out.push({
      id: encodeToken({
        p: "subsource",
        u: `https://api.subsource.net/api/v1/subtitles/${sid}/download`,
        s: query.season,
        e: query.episode,
        k: apiKey,
      }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
      format: "srt",
      releaseName: rel || undefined,
      sourceName: "SubSource",
      downloads: item.downloads,
      hearingImpaired: Boolean(item.hearingImpaired),
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// 4. TurkceAltyazi.org (API key-free, Turkish + English)
// ---------------------------------------------------------------------------
async function searchTurkceAltyazi(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  // Prefer running when Turkish is requested or no language filter is enforced.
  if (prefs.length > 0 && !prefs.includes("tr") && !prefs.includes("en")) {
    return [];
  }

  const imdb = normalizeImdb(query.externalIds?.imdb);
  const terms = [query.originalTitle, query.title].filter((v): v is string => Boolean(v && v.trim()));
  let movPath: string | null = null;

  for (const term of terms) {
    const findResp = await vessel.http.get(
      `https://turkcealtyazi.org/find.php?cat=sub&find=${encodeURIComponent(term)}`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (findResp.status >= 400) continue;
    const doc = vessel.html.parse(findResp.text());
    const links = doc.select('a[href^="/mov/"]');
    if (imdb) {
      const num = imdb.replace(/^tt/, "");
      const exact = links.find((a) => (a.attr("href") || "").includes(`/${num}/`));
      if (exact) {
        movPath = exact.attr("href");
        break;
      }
    }
    if (!movPath && links.length > 0) {
      movPath = links[0].attr("href");
      if (movPath) break;
    }
  }

  if (!movPath) return [];
  const movUrl = `https://turkcealtyazi.org${movPath}`;
  const movResp = await vessel.http.get(movUrl, {
    headers: { "User-Agent": UA, Referer: "https://turkcealtyazi.org/" },
    timeoutMs: 12000,
  });
  if (movResp.status >= 400) return [];

  const movDoc = vessel.html.parse(movResp.text());
  const rows = movDoc.select(".altyazi-list-wrapper > div > div[class*='altsonsez']");
  const epRx = episodeRegex(query.season, query.episode);
  const out: SubtitleCandidate[] = [];

  for (const row of rows) {
    const link = row.selectOne('a[href^="/sub/"]');
    const href = link?.attr("href");
    if (!href) continue;

    const flagImg = row.selectOne(".aldil span, .aldil img");
    const langHint = flagImg?.attr("title") || flagImg?.attr("alt") || row.selectOne(".aldil")?.text() || "tr";
    const lang = langHint.toLowerCase().includes("eng") ? "en" : normalizeLang(langHint || "tr");
    if (!matchesPreferred(lang, prefs)) continue;

    const rowText = row.text().replace(/\s+/g, " ").trim();
    if (query.mediaType === "series" && epRx) {
      // Allow season packs or matching episode tags when present.
      const hasSeasonOnly =
        query.season &&
        new RegExp(`\\b(s0*${query.season}|sezon\\s*0*${query.season})\\b`, "i").test(rowText) &&
        !/s\d+e\d+|\d+x\d+/i.test(rowText);
      if (!epRx.test(rowText) && !hasSeasonOnly) continue;
    }

    const ripEl = row.selectOne(".alrip, .altyazi-rip");
    const translator = row.selectOne(".alcevirmen")?.text().trim() || "";
    const rel = (ripEl?.text().trim() || link?.attr("title") || translator || "TurkceAltyazi").trim();
    const subUrl = `https://turkcealtyazi.org${href}`;

    out.push({
      id: encodeToken({
        p: "ta",
        u: subUrl,
        s: query.season,
        e: query.episode,
      }),
      lang,
      label: `${langDisplayName(lang)} · ${rel}`,
      format: "srt",
      releaseName: rel,
      sourceName: "TurkceAltyazi",
    });
    if (out.length >= 25) break;
  }

  return out;
}

async function resolveTurkceAltyazi(tok: ResolveToken): Promise<ResolvedSubtitle> {
  const pageResp = await vessel.http.get(tok.u, {
    headers: { "User-Agent": UA, Referer: "https://turkcealtyazi.org/" },
    timeoutMs: 12000,
  });
  if (pageResp.status >= 400) {
    throw new Error(`TurkceAltyazi page HTTP ${pageResp.status}`);
  }
  const doc = vessel.html.parse(pageResp.text());
  const idid = doc.selectOne('input[name="idid"]')?.attr("value") || "";
  const altid = doc.selectOne('input[name="altid"]')?.attr("value") || "";
  const sidid = doc.selectOne('input[name="sidid"]')?.attr("value") || "";
  if (!idid || !altid || !sidid) {
    throw new Error("TurkceAltyazi download token fields not found");
  }

  const body = new URLSearchParams({ idid, altid, sidid }).toString();
  const dlResp = await vessel.http.post("https://turkcealtyazi.org/ind", body, {
    headers: {
      "User-Agent": UA,
      Referer: tok.u,
      Origin: "https://turkcealtyazi.org",
      "Content-Type": "application/x-www-form-urlencoded",
    },
    timeoutMs: 15000,
  });
  if (dlResp.status >= 400) {
    throw new Error(`TurkceAltyazi download HTTP ${dlResp.status}`);
  }
  const entries = vessel.archive.unzipEntries(dlResp.bytes());
  const picked = pickBestZipEntry(entries, tok.s, tok.e);
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

// ---------------------------------------------------------------------------
// 5. YIFY Subtitles (Movies, API key-free)
// ---------------------------------------------------------------------------
async function searchYify(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  if (query.mediaType !== "movie") return [];
  const imdb = normalizeImdb(query.externalIds?.imdb);
  let movieUrl = imdb ? `https://yifysubtitles.ch/movie-imdb/${imdb}` : "";

  if (!movieUrl) {
    const qText = (query.originalTitle || query.title || "").trim();
    if (!qText) return [];
    const sResp = await vessel.http.get(
      `https://yifysubtitles.ch/search?q=${encodeURIComponent(qText)}`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (sResp.status >= 400) return [];
    const sDoc = vessel.html.parse(sResp.text());
    const hit = sDoc.selectOne('a[href^="/movie-imdb/"]')?.attr("href");
    if (!hit) return [];
    movieUrl = `https://yifysubtitles.ch${hit}`;
  }

  const resp = await vessel.http.get(movieUrl, {
    headers: { "User-Agent": UA },
    timeoutMs: 12000,
  });
  if (resp.status >= 400) return [];

  const doc = vessel.html.parse(resp.text());
  const rows = doc.select("table.other-subs tbody tr");
  const out: SubtitleCandidate[] = [];

  for (const row of rows) {
    const langText = row.selectOne(".sub-lang")?.text() || "";
    const lang = normalizeLang(langText);
    if (!matchesPreferred(lang, prefs)) continue;

    const a = row.selectOne('a[href^="/subtitles/"]');
    const href = a?.attr("href");
    if (!href) continue;

    const rel = (a?.text() || "").replace(/^subtitle\s+/i, "").trim();
    const zipPath = href.replace(/^\/subtitles\//, "/subtitle/") + ".zip";
    const refererUrl = `https://yifysubtitles.ch${href}`;
    const zipUrl = `https://yifysubtitles.ch${zipPath}`;

    out.push({
      id: encodeToken({ p: "zip", u: zipUrl, r: refererUrl }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
      format: "srt",
      releaseName: rel || undefined,
      sourceName: "YIFY",
    });
    if (out.length >= 25) break;
  }

  return out;
}

// ---------------------------------------------------------------------------
// 6. Addic7ed (TV Series, API key-free)
// ---------------------------------------------------------------------------
async function searchAddic7ed(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  if (query.mediaType !== "series" || !query.season || !query.episode) return [];
  const qText = (query.originalTitle || query.title || "").trim();
  if (!qText) return [];

  const srchResp = await vessel.http.get(
    `https://www.addic7ed.com/srch.php?search=${encodeURIComponent(qText)}&Submit=Search`,
    { headers: { "User-Agent": UA }, timeoutMs: 12000 },
  );
  if (srchResp.status >= 400) return [];

  const srchDoc = vessel.html.parse(srchResp.text());
  let epUrl = "";
  const seasonEpRx = new RegExp(`/${query.season}/${query.episode}/`, "i");
  for (const a of srchDoc.select('a[href*="serie/"]')) {
    const href = a.attr("href") || "";
    if (seasonEpRx.test(href)) {
      epUrl = href.startsWith("http") ? href : `https://www.addic7ed.com/${href.replace(/^\/+/, "")}`;
      break;
    }
  }
  if (!epUrl) {
    const firstShow = srchDoc.selectOne('a[href*="serie/"]')?.attr("href");
    if (!firstShow) return [];
    const m = /serie\/([^/]+)\//i.exec(firstShow);
    if (!m) return [];
    epUrl = `https://www.addic7ed.com/serie/${m[1]}/${query.season}/${query.episode}/1`;
  }

  const epResp = await vessel.http.get(epUrl, {
    headers: { "User-Agent": UA, Referer: "https://www.addic7ed.com/" },
    timeoutMs: 12000,
  });
  if (epResp.status >= 400) return [];

  const epDoc = vessel.html.parse(epResp.text());
  const out: SubtitleCandidate[] = [];
  for (const block of epDoc.select("div#container95m, table.tabel95")) {
    const langNode = block.selectOne("td.language");
    if (!langNode) continue;
    const lang = normalizeLang(langNode.text().replace(/view|edit/gi, ""));
    if (!matchesPreferred(lang, prefs)) continue;

    const dl = block.selectOne('a.buttonDownload[href*="original/"], a.buttonDownload[href*="updated/"]');
    const dlHref = dl?.attr("href");
    if (!dlHref) continue;

    const dlUrl = dlHref.startsWith("http")
      ? dlHref
      : `https://www.addic7ed.com${dlHref.startsWith("/") ? "" : "/"}${dlHref}`;
    const version = block.selectOne("td.NewsTitle")?.text().replace(/Version\s*/i, "").trim() || "";
    const hi = block.text().toLowerCase().includes("hearing impaired");

    out.push({
      id: encodeToken({ p: "direct", u: dlUrl, r: epUrl, f: "srt" }),
      lang,
      label: version ? `${langDisplayName(lang)} · ${version}` : langDisplayName(lang),
      format: "srt",
      releaseName: version || undefined,
      sourceName: "Addic7ed",
      hearingImpaired: hi,
    });
    if (out.length >= 25) break;
  }

  return out;
}

// ---------------------------------------------------------------------------
// 7. TVSubtitles.net (TV Series, API key-free)
// ---------------------------------------------------------------------------
async function searchTvSubtitles(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  if (query.mediaType !== "series" || !query.season || !query.episode) return [];
  const qText = (query.originalTitle || query.title || "").trim();
  if (!qText) return [];

  const sResp = await vessel.http.post(
    "https://www.tvsubtitles.net/search2.php",
    `qs=${encodeURIComponent(qText)}`,
    {
      headers: {
        "User-Agent": UA,
        Referer: "https://www.tvsubtitles.net/",
        "Content-Type": "application/x-www-form-urlencoded",
      },
      timeoutMs: 12000,
    },
  );
  if (sResp.status >= 400) return [];

  const sDoc = vessel.html.parse(sResp.text());
  const normQuery = normalizeText(qText);
  let showHref = "";
  for (const a of sDoc.select('a[href^="/tvshow-"]')) {
    const href = a.attr("href") || "";
    if (!/^\/tvshow-\d+\.html$/.test(href)) continue;
    const text = normalizeText(a.text());
    if (!showHref) showHref = href;
    if (text === normQuery || text.includes(normQuery)) {
      showHref = href;
      break;
    }
  }
  if (!showHref) return [];

  const showIdMatch = /\/tvshow-(\d+)\.html/.exec(showHref);
  if (!showIdMatch) return [];
  const seasonUrl = `https://www.tvsubtitles.net/tvshow-${showIdMatch[1]}-${query.season}.html`;
  const seasonResp = await vessel.http.get(seasonUrl, {
    headers: { "User-Agent": UA, Referer: "https://www.tvsubtitles.net/" },
    timeoutMs: 12000,
  });
  if (seasonResp.status >= 400) return [];

  const seasonDoc = vessel.html.parse(seasonResp.text());
  const targetEp = `${query.season}x${pad2(query.episode)}`;
  let epHref = "";
  for (const row of seasonDoc.select("table tr")) {
    const text = row.text();
    if (text.includes(targetEp) || text.includes(`${query.season}x${query.episode}`)) {
      const link = row.selectOne('a[href*="episode-"]');
      if (link?.attr("href")) {
        epHref = link.attr("href")!;
        break;
      }
    }
  }
  if (!epHref) return [];

  const epUrl = `https://www.tvsubtitles.net/${epHref.replace(/^\/+/, "")}`;
  const epResp = await vessel.http.get(epUrl, {
    headers: { "User-Agent": UA, Referer: seasonUrl },
    timeoutMs: 12000,
  });
  if (epResp.status >= 400) return [];

  const epDoc = vessel.html.parse(epResp.text());
  const out: SubtitleCandidate[] = [];
  for (const a of epDoc.select('a[href*="subtitle-"]')) {
    const href = a.attr("href") || "";
    const subMatch = /subtitle-(\d+)\.html/.exec(href);
    if (!subMatch) continue;

    const flagSrc = a.selectOne("img")?.attr("src") || "";
    const langMatch = /flags\/([a-z]{2})\./i.exec(flagSrc);
    const lang = normalizeLang(langMatch?.[1] || "en");
    if (!matchesPreferred(lang, prefs)) continue;

    const rel = a.text().replace(/\s+/g, " ").trim();
    out.push({
      id: encodeToken({
        p: "tvsub",
        u: `https://www.tvsubtitles.net/download-${subMatch[1]}.html`,
        r: `https://www.tvsubtitles.net/subtitle-${subMatch[1]}.html`,
        s: query.season,
        e: query.episode,
      }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
      format: "srt",
      releaseName: rel || undefined,
      sourceName: "TVSubtitles",
    });
    if (out.length >= 25) break;
  }
  return out;
}

async function resolveTvSubtitles(tok: ResolveToken): Promise<ResolvedSubtitle> {
  const pageResp = await vessel.http.get(tok.u, {
    headers: { "User-Agent": UA, Referer: tok.r || "https://www.tvsubtitles.net/" },
    timeoutMs: 12000,
  });
  if (pageResp.status >= 400) {
    throw new Error(`TVSubtitles download page HTTP ${pageResp.status}`);
  }
  const html = pageResp.text();
  const s1 = /var\s+s1\s*=\s*['"]([^'"]+)['"]/.exec(html)?.[1] || "";
  const s2 = /var\s+s2\s*=\s*['"]([^'"]+)['"]/.exec(html)?.[1] || "";
  const s3 = /var\s+s3\s*=\s*['"]([^'"]+)['"]/.exec(html)?.[1] || "";
  const s4 = /var\s+s4\s*=\s*['"]([^'"]+)['"]/.exec(html)?.[1] || "";
  const relPath = `${s1}${s2}${s3}${s4}`.replace(/^\/+/, "");
  if (!relPath) {
    throw new Error("TVSubtitles ZIP path not found");
  }

  const zipResp = await vessel.http.get(`https://www.tvsubtitles.net/${relPath}`, {
    headers: { "User-Agent": UA, Referer: tok.u },
    timeoutMs: 15000,
  });
  if (zipResp.status >= 400) {
    throw new Error(`TVSubtitles ZIP HTTP ${zipResp.status}`);
  }
  const entries = vessel.archive.unzipEntries(zipResp.bytes());
  const picked = pickBestZipEntry(entries, tok.s, tok.e);
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

// ---------------------------------------------------------------------------
// 8. Kitsunekko (Anime, API key-free)
// ---------------------------------------------------------------------------
async function searchKitsunekko(query: SubtitleQuery, prefs: string[]): Promise<SubtitleCandidate[]> {
  if (prefs.length > 0 && !prefs.includes("ja") && !prefs.includes("en")) {
    return [];
  }
  const qNorm = normalizeText(query.originalTitle || query.title || "");
  if (!qNorm || qNorm.length < 2) return [];

  const buckets: Array<{ lang: string; dir: string }> = [];
  if (!prefs.length || prefs.includes("ja")) {
    buckets.push({ lang: "ja", dir: "subtitles%2Fjapanese%2F" });
  }
  if (!prefs.length || prefs.includes("en")) {
    buckets.push({ lang: "en", dir: "subtitles%2Fenglish%2F" });
  }

  const out: SubtitleCandidate[] = [];
  const words = qNorm.split(" ").filter((w) => w.length >= 2);
  const epNum = query.episode;

  for (const bucket of buckets) {
    const dirResp = await vessel.http.get(
      `https://kitsunekko.net/dirlist.php?dir=${bucket.dir}`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (dirResp.status >= 400) continue;

    const doc = vessel.html.parse(dirResp.text());
    let matchedHref = "";
    for (const a of doc.select('a[href^="/dirlist.php?dir="]')) {
      const href = a.attr("href") || "";
      const nameNorm = normalizeText(a.text());
      if (!nameNorm) continue;
      if (nameNorm === qNorm || nameNorm.includes(qNorm) || (words.length >= 2 && words.every((w) => nameNorm.includes(w)))) {
        matchedHref = href;
        break;
      }
    }
    if (!matchedHref) continue;

    const showResp = await vessel.http.get(`https://kitsunekko.net${matchedHref}`, {
      headers: { "User-Agent": UA },
      timeoutMs: 12000,
    });
    if (showResp.status >= 400) continue;

    const showDoc = vessel.html.parse(showResp.text());
    for (const a of showDoc.select('a[href^="subtitles/"]')) {
      const href = a.attr("href") || "";
      const lower = href.toLowerCase();
      const isZip = lower.endsWith(".zip");
      if (!isZip && !isSupportedSubFile(lower)) continue;

      const fileName = a.text().trim() || href.split("/").pop() || "subtitle";
      if (epNum && !isZip) {
        const epRx = new RegExp(`(?:^|[^0-9])0*${epNum}(?:[^0-9]|$)`);
        if (!epRx.test(fileName)) continue;
      }

      const fileUrl = `https://kitsunekko.net/${href.replace(/^\/+/, "")}`;
      out.push({
        id: encodeToken({
          p: isZip ? "zip" : "direct",
          u: fileUrl,
          f: detectFormat(fileName, "ass"),
          s: query.season,
          e: query.episode,
        }),
        lang: bucket.lang,
        label: `${langDisplayName(bucket.lang)} · ${fileName}`,
        format: detectFormat(fileName, "ass"),
        releaseName: fileName,
        sourceName: "Kitsunekko",
      });
      if (out.length >= 25) break;
    }
  }

  return out;
}

// ---------------------------------------------------------------------------
// Generic Resolver
// ---------------------------------------------------------------------------
async function resolveDirect(tok: ResolveToken): Promise<ResolvedSubtitle> {
  const headers: Record<string, string> = { "User-Agent": UA };
  if (tok.r) headers["Referer"] = tok.r;
  const resp = await vessel.http.get(tok.u, { headers, timeoutMs: 15000 });
  if (resp.status >= 400) {
    throw new Error(`Subtitle download HTTP ${resp.status}`);
  }
  const content = resp.text();
  return {
    format: tok.f || detectFormat(tok.u, "srt"),
    source: { kind: "inline_content", content },
  };
}

async function resolveZip(tok: ResolveToken, extraHeaders?: Record<string, string>): Promise<ResolvedSubtitle> {
  const headers: Record<string, string> = { "User-Agent": UA, ...(extraHeaders ?? {}) };
  if (tok.r) headers["Referer"] = tok.r;
  const resp = await vessel.http.get(tok.u, { headers, timeoutMs: 15000 });
  if (resp.status >= 400) {
    throw new Error(`Subtitle archive HTTP ${resp.status}`);
  }
  const entries = vessel.archive.unzipEntries(resp.bytes());
  const picked = pickBestZipEntry(entries, tok.s, tok.e);
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

export default vessel.definePlugin({
  async searchSubtitles(query: SubtitleQuery): Promise<SubtitleCandidate[]> {
    const prefs = preferredSet(query);
    const tasks: Array<{ name: string; run: () => Promise<SubtitleCandidate[]> }> = [];

    if (await readBoolSetting("enableOpenSubtitles", true)) {
      tasks.push({ name: "OpenSubtitles", run: () => searchOpenSubtitles(query, prefs) });
    }
    if (await readBoolSetting("enableSubdl", true)) {
      tasks.push({ name: "SubDL", run: () => searchSubdl(query, prefs) });
    }
    if (await readBoolSetting("enableSubsource", true)) {
      tasks.push({ name: "SubSource", run: () => searchSubsource(query, prefs) });
    }
    if (await readBoolSetting("enableTurkceAltyazi", true)) {
      tasks.push({ name: "TurkceAltyazi", run: () => searchTurkceAltyazi(query, prefs) });
    }
    if (await readBoolSetting("enableYify", true)) {
      tasks.push({ name: "YIFY", run: () => searchYify(query, prefs) });
    }
    if (await readBoolSetting("enableAddic7ed", true)) {
      tasks.push({ name: "Addic7ed", run: () => searchAddic7ed(query, prefs) });
    }
    if (await readBoolSetting("enableTvSubtitles", true)) {
      tasks.push({ name: "TVSubtitles", run: () => searchTvSubtitles(query, prefs) });
    }
    if (await readBoolSetting("enableKitsunekko", true)) {
      tasks.push({ name: "Kitsunekko", run: () => searchKitsunekko(query, prefs) });
    }

    if (!tasks.length) {
      throw new Error("All providers inside Subtitles Container are disabled.");
    }

    const settled = await Promise.allSettled(tasks.map((t) => t.run()));
    const combined: SubtitleCandidate[] = [];
    const errors: string[] = [];

    for (let i = 0; i < settled.length; i++) {
      const res = settled[i];
      if (res.status === "fulfilled") {
        combined.push(...res.value);
      } else {
        const msg = res.reason instanceof Error ? res.reason.message : String(res.reason);
        if (msg.includes("[VesselDomainBlockedError]")) {
          throw res.reason;
        }
        errors.push(`${tasks[i].name}: ${msg}`);
      }
    }

    if (errors.length > 0) {
      console.warn(`Subtitles Container partial issues -> ${errors.join(" | ")}`);
    }

    if (combined.length === 0 && errors.length === tasks.length) {
      throw new Error(errors.join(" | "));
    }

    // Rank preferred languages first, then download count.
    combined.sort((a, b) => {
      const aIdx = prefs.indexOf(a.lang);
      const bIdx = prefs.indexOf(b.lang);
      const aRank = aIdx >= 0 ? aIdx : 99;
      const bRank = bIdx >= 0 ? bIdx : 99;
      if (aRank !== bRank) return aRank - bRank;
      return (b.downloads ?? 0) - (a.downloads ?? 0);
    });

    return combined;
  },

  async resolveSubtitle(subtitleId: string): Promise<ResolvedSubtitle> {
    const tok = decodeToken(subtitleId);
    switch (tok.p) {
      case "os_api":
        return resolveOpenSubtitlesApi(tok);
      case "ta":
        return resolveTurkceAltyazi(tok);
      case "tvsub":
        return resolveTvSubtitles(tok);
      case "subsource":
        return resolveZip(tok, tok.k ? { "X-API-Key": tok.k } : undefined);
      case "zip":
        return resolveZip(tok);
      case "direct":
      default:
        return resolveDirect(tok);
    }
  },
});
