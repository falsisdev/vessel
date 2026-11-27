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
  k?: string;
  fileId?: number;
  ua?: string;
}

interface SearchContext {
  query: SubtitleQuery;
  prefs: string[];
  imdb: string | null;
  titles: string[];
  year?: number;
}

const UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const OS_REST_UA = "TemporaryUserAgent";

const LANG_NAME_TO_CODE: Record<string, string> = {
  turkish: "tr",
  tur: "tr",
  "türkçe": "tr",
  turkce: "tr",
  tr: "tr",
  english: "en",
  eng: "en",
  en: "en",
  spanish: "es",
  spa: "es",
  es: "es",
  french: "fr",
  fre: "fr",
  fra: "fr",
  fr: "fr",
  german: "de",
  ger: "de",
  deu: "de",
  de: "de",
  italian: "it",
  ita: "it",
  it: "it",
  portuguese: "pt",
  por: "pt",
  pt: "pt",
  "brazilian portuguese": "pt-br",
  "portuguese (brazil)": "pt-br",
  brazilian: "pt-br",
  pob: "pt-br",
  "pt-br": "pt-br",
  russian: "ru",
  rus: "ru",
  ru: "ru",
  japanese: "ja",
  jpn: "ja",
  ja: "ja",
  arabic: "ar",
  ara: "ar",
  ar: "ar",
  persian: "fa",
  farsi: "fa",
  "farsi/persian": "fa",
  per: "fa",
  fas: "fa",
  fa: "fa",
  chinese: "zh",
  chi: "zh",
  zho: "zh",
  zh: "zh",
  korean: "ko",
  kor: "ko",
  ko: "ko",
  dutch: "nl",
  dut: "nl",
  nld: "nl",
  nl: "nl",
  polish: "pl",
  pol: "pl",
  pl: "pl",
  romanian: "ro",
  rum: "ro",
  ron: "ro",
  ro: "ro",
  greek: "el",
  ell: "el",
  gre: "el",
  el: "el",
  ukrainian: "uk",
  ukr: "uk",
  uk: "uk",
  indonesian: "id",
  ind: "id",
  id: "id",
  vietnamese: "vi",
  vie: "vi",
  vi: "vi",
  hindi: "hi",
  hin: "hi",
  hi: "hi",
  swedish: "sv",
  swe: "sv",
  sv: "sv",
  norwegian: "no",
  nor: "no",
  nob: "no",
  no: "no",
  danish: "da",
  dan: "da",
  da: "da",
  finnish: "fi",
  fin: "fi",
  fi: "fi",
  czech: "cs",
  cze: "cs",
  ces: "cs",
  cs: "cs",
  hungarian: "hu",
  hun: "hu",
  hu: "hu",
  bulgarian: "bg",
  bul: "bg",
  bg: "bg",
  croatian: "hr",
  hrv: "hr",
  hr: "hr",
  serbian: "sr",
  srp: "sr",
  sr: "sr",
  slovenian: "sl",
  slv: "sl",
  sl: "sl",
  estonian: "et",
  est: "et",
  et: "et",
  hebrew: "he",
  heb: "he",
  he: "he",
  albanian: "sq",
  alb: "sq",
  sqi: "sq",
  sq: "sq",
  macedonian: "mk",
  mac: "mk",
  mkd: "mk",
  mk: "mk",
};

const CODE_TO_3: Record<string, string> = {
  tr: "tur",
  en: "eng",
  es: "spa",
  fr: "fre",
  de: "ger",
  it: "ita",
  pt: "por",
  "pt-br": "pob",
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
  sv: "swe",
  no: "nor",
  da: "dan",
  fi: "fin",
  cs: "cze",
  hu: "hun",
  bg: "bul",
  hr: "hrv",
  sr: "srp",
  sl: "slv",
  et: "est",
  he: "heb",
  sq: "alb",
  mk: "mac",
};

const CODE_TO_SUBDL_SLUG: Record<string, string> = {
  tr: "turkish",
  en: "english",
  es: "spanish",
  fr: "french",
  de: "german",
  it: "italian",
  pt: "portuguese",
  "pt-br": "brazillian-portuguese",
  ru: "russian",
  ja: "japanese",
  ar: "arabic",
  fa: "farsi_persian",
  zh: "chinese-bg-code",
  ko: "korean",
  nl: "dutch",
  pl: "polish",
  ro: "romanian",
  el: "greek",
  uk: "ukrainian",
  id: "indonesian",
  vi: "vietnamese",
  hi: "hindi",
  sv: "swedish",
  no: "norwegian",
  da: "danish",
  fi: "finnish",
  cs: "czech",
  hu: "hungarian",
  bg: "bulgarian",
  hr: "croatian",
  sr: "serbian",
  he: "hebrew",
};

function isDomainBlockedError(err: unknown): boolean {
  const msg = err instanceof Error ? err.message : String(err);
  return msg.includes("[VesselDomainBlockedError]");
}

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
  const clean = raw
    .trim()
    .toLowerCase()
    .replace(/_/g, "-")
    .replace(/\s+/g, " ");
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
    nl: "Nederlands",
    pl: "Polski",
    ro: "Română",
    el: "Ελληνικά",
    uk: "Українська",
    deu: "Deutsch",
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

function foldTurkish(s: string): string {
  return s
    .replace(/[çÇ]/g, "c")
    .replace(/[ğĞ]/g, "g")
    .replace(/[ıİ]/g, "i")
    .replace(/[öÖ]/g, "o")
    .replace(/[şŞ]/g, "s")
    .replace(/[üÜ]/g, "u");
}

function normalizeText(s: string): string {
  return foldTurkish(s)
    .toLowerCase()
    .replace(/\(\s*(?:19|20)\d{2}\s*\)|\[\s*(?:19|20)\d{2}\s*\]/g, " ")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function titleMatchScore(candidateRaw: string, queryTerms: string[]): number {
  const cand = normalizeText(candidateRaw);
  if (!cand) return 0;
  const candWords = cand.split(" ").filter(Boolean);
  let best = 0;

  for (const rawTerm of queryTerms) {
    const q = normalizeText(rawTerm);
    if (!q) continue;
    if (cand === q) return 100;
    if (cand.startsWith(`${q} `) || q.startsWith(`${cand} `)) {
      best = Math.max(best, 85);
      continue;
    }
    const qWords = q.split(" ").filter((w) => w.length >= 2 || /^\d+$/.test(w));
    if (!qWords.length) {
      if (cand.includes(q)) best = Math.max(best, 75);
      continue;
    }
    const allPresent = qWords.every((w) => candWords.includes(w));
    if (allPresent) {
      const extraRatio = candWords.length / qWords.length;
      if (extraRatio <= 2.2) {
        best = Math.max(best, Math.round(80 - (candWords.length - qWords.length) * 8));
      } else {
        best = Math.max(best, 40);
      }
    }
  }
  return best;
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
    `(?:s0*${s}[\\s._|\\-/]*[eb]0*${e}\\b|\\b${s}x0*${e}\\b|\\b0*${s2}x0*${e2}\\b|\\bep\\.?\\s*0*${e}\\b)`,
    "i",
  );
}

function decodeArchiveOrText(
  bytes: Uint8Array,
  season?: number,
  episode?: number,
  fallbackFormat: SubtitleFormat = "srt",
): { format: SubtitleFormat; content: string } {
  if (bytes.length >= 2 && bytes[0] === 0x50 && bytes[1] === 0x4b) {
    const entries = vessel.archive.unzipEntries(bytes);
    return pickBestZipEntry(entries, season, episode);
  }
  if (bytes.length >= 2 && bytes[0] === 0x1f && bytes[1] === 0x8b) {
    const plainBytes = vessel.archive.gunzip(bytes);
    return {
      format: fallbackFormat,
      content: vessel.archive.decodeText(plainBytes, "auto"),
    };
  }
  return {
    format: fallbackFormat,
    content: vessel.archive.decodeText(bytes, "auto"),
  };
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
  if (/^tt\d+$/i.test(trimmed)) return trimmed.toLowerCase();
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

function pushUniqueTitle(list: string[], candidate?: string | null): void {
  if (!candidate) return;
  const trimmed = candidate.trim();
  if (!trimmed) return;
  const norm = trimmed.toLowerCase();
  if (!list.some((t) => t.toLowerCase() === norm)) {
    list.push(trimmed);
  }
}

// ---------------------------------------------------------------------------
// 0. Automatic IMDb & English Title Resolution (for plugins without IMDb IDs)
// ---------------------------------------------------------------------------
async function buildSearchContext(query: SubtitleQuery, prefs: string[]): Promise<SearchContext> {
  let imdb = normalizeImdb(query.externalIds?.imdb);
  const titles: string[] = [];
  pushUniqueTitle(titles, query.originalTitle);
  pushUniqueTitle(titles, query.title);
  // Also add stripped clean variant if title has trailing year or brackets e.g. "Inception (2010)"
  for (const t of [...titles]) {
    const cleaned = t
      .replace(/\(\s*(?:19|20)\d{2}\s*\)|\[\s*(?:19|20)\d{2}\s*\]/g, "")
      .trim();
    pushUniqueTitle(titles, cleaned);
  }

  let resolvedYear = query.year;

  // Resolve IMDb ID & canonical English title from Cinemeta / IMDb Suggest when missing.
  const searchSeed = titles[0] || "";
  if (!imdb && searchSeed.length >= 2) {
    const cmType = query.mediaType === "series" ? "series" : "movie";
    try {
      const cmResp = await vessel.http.get(
        `https://v3-cinemeta.strem.io/catalog/${cmType}/top/search=${encodeURIComponent(searchSeed)}.json`,
        { headers: { "User-Agent": UA, Accept: "application/json" }, timeoutMs: 4500 },
      );
      if (cmResp.status < 400) {
        const cmJson = cmResp.json<{
          metas?: Array<{
            id?: string;
            imdb_id?: string;
            name?: string;
            releaseInfo?: string;
            year?: number;
          }>;
        }>();
        const metas = cmJson.metas ?? [];
        let bestMeta: (typeof metas)[number] | null = null;
        let bestScore = -1;
        for (let i = 0; i < Math.min(metas.length, 8); i++) {
          const m = metas[i];
          const mid = normalizeImdb(m.imdb_id || m.id);
          if (!mid) continue;
          const mName = (m.name || "").trim();
          const yrMatch = /(\d{4})/.exec(m.releaseInfo || "");
          const mYear = m.year ?? (yrMatch ? Number(yrMatch[1]) : undefined);
          let score = titleMatchScore(mName, titles);
          if (resolvedYear && mYear) {
            if (Math.abs(resolvedYear - mYear) <= 1) score += 40;
            else score -= 35;
          }
          // Cinemeta ranks translated-title hits well at position 0.
          if (i === 0 && score === 0) score = 20;
          if (score > bestScore) {
            bestScore = score;
            bestMeta = m;
          }
        }
        if (bestMeta && bestScore >= 20) {
          imdb = normalizeImdb(bestMeta.imdb_id || bestMeta.id);
          pushUniqueTitle(titles, bestMeta.name);
          if (!resolvedYear) {
            const yrMatch = /(\d{4})/.exec(bestMeta.releaseInfo || "");
            if (yrMatch) resolvedYear = Number(yrMatch[1]);
          }
        }
      }
    } catch (err) {
      if (isDomainBlockedError(err)) throw err;
    }
  }

  // Fallback to IMDb suggest API if Cinemeta did not find an IMDb ID.
  if (!imdb && searchSeed.length >= 2) {
    const firstChar = foldTurkish(searchSeed).toLowerCase().replace(/[^a-z0-9]/g, "")[0] || "a";
    try {
      const imdbResp = await vessel.http.get(
        `https://v3.sg.media-imdb.com/suggestion/${firstChar}/${encodeURIComponent(searchSeed)}.json`,
        { headers: { "User-Agent": UA, Accept: "application/json" }, timeoutMs: 4500 },
      );
      if (imdbResp.status < 400) {
        const imdbJson = imdbResp.json<{
          d?: Array<{ id?: string; l?: string; qid?: string; y?: number }>;
        }>();
        const items = imdbJson.d ?? [];
        const wantSeries = query.mediaType === "series";
        let bestItem: (typeof items)[number] | null = null;
        let bestScore = -1;
        for (let i = 0; i < Math.min(items.length, 8); i++) {
          const it = items[i];
          const iid = normalizeImdb(it.id);
          if (!iid) continue;
          const qid = (it.qid || "").toLowerCase();
          const isTv = qid.includes("tvseries") || qid.includes("tvminiseries");
          const isMov = qid === "movie" || qid === "tvmovie";
          if (wantSeries && isMov) continue;
          if (!wantSeries && isTv) continue;
          let score = titleMatchScore(it.l || "", titles);
          if (resolvedYear && it.y) {
            if (Math.abs(resolvedYear - it.y) <= 1) score += 40;
            else score -= 35;
          }
          if (i === 0 && score === 0) score = 20;
          if (score > bestScore) {
            bestScore = score;
            bestItem = it;
          }
        }
        if (bestItem && bestScore >= 20) {
          imdb = normalizeImdb(bestItem.id);
          pushUniqueTitle(titles, bestItem.l);
          if (!resolvedYear && bestItem.y) resolvedYear = bestItem.y;
        }
      }
    } catch (err) {
      if (isDomainBlockedError(err)) throw err;
    }
  }

  return {
    query,
    prefs,
    imdb,
    titles,
    year: resolvedYear,
  };
}

// ---------------------------------------------------------------------------
// 1. OpenSubtitles (Official REST API v1 + Key-free rest.opensubtitles.org + Stremio v3)
// ---------------------------------------------------------------------------
async function searchOpenSubtitlesRestLegacy(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  const isSeries = ctx.query.mediaType === "series";
  const langTargets: Array<string | null> = [];
  if (ctx.prefs.length > 0) {
    for (const p of ctx.prefs.slice(0, 3)) {
      const code3 = CODE_TO_3[p];
      if (code3 && !langTargets.includes(code3)) langTargets.push(code3);
    }
  }
  if (!langTargets.length) {
    langTargets.push(null);
  }

  const urls: string[] = [];
  const baseTitle = ctx.titles[0] || ctx.query.title;
  const imdbNum = ctx.imdb ? ctx.imdb.replace(/^tt/, "") : null;

  for (const lang3 of langTargets) {
    // OpenSubtitles legacy REST requires path segments in alphabetical order!
    const segs: string[] = [];
    if (isSeries && ctx.query.episode) segs.push(`episode-${ctx.query.episode}`);
    if (imdbNum) {
      segs.push(`imdbid-${imdbNum}`);
    } else if (baseTitle) {
      segs.push(`query-${encodeURIComponent(baseTitle.toLowerCase())}`);
    } else {
      continue;
    }
    if (isSeries && ctx.query.season) segs.push(`season-${ctx.query.season}`);
    if (lang3) segs.push(`sublanguageid-${lang3}`);
    urls.push(`https://rest.opensubtitles.org/search/${segs.join("/")}`);
  }

  if (!urls.length) return [];

  const responses = await Promise.allSettled(
    urls.map((u) =>
      vessel.http.get(u, {
        headers: { "User-Agent": OS_REST_UA, Accept: "application/json" },
        timeoutMs: 12000,
      }),
    ),
  );

  const out: SubtitleCandidate[] = [];
  const seenIds = new Set<string>();

  for (const r of responses) {
    if (r.status === "rejected") {
      if (isDomainBlockedError(r.reason)) throw r.reason;
      continue;
    }
    if (r.value.status >= 400) continue;
    let items: Array<{
      IDSubtitleFile?: string;
      SubFileName?: string;
      SubFormat?: string;
      SubDownloadsCnt?: string;
      SubHearingImpaired?: string;
      SubDownloadLink?: string;
      ISO639?: string;
      LanguageName?: string;
      MovieReleaseName?: string;
      MovieKind?: string;
      SeriesSeason?: string;
      SeriesEpisode?: string;
    }> = [];
    try {
      items = r.value.json();
    } catch {
      continue;
    }
    if (!Array.isArray(items)) continue;

    for (const item of items) {
      const dlLink = item.SubDownloadLink;
      if (!dlLink) continue;
      if (item.IDSubtitleFile && seenIds.has(item.IDSubtitleFile)) continue;
      if (item.IDSubtitleFile) seenIds.add(item.IDSubtitleFile);

      if (!imdbNum && isSeries) {
        if (ctx.query.season && item.SeriesSeason && Number(item.SeriesSeason) !== ctx.query.season) {
          continue;
        }
        if (ctx.query.episode && item.SeriesEpisode && Number(item.SeriesEpisode) !== ctx.query.episode) {
          continue;
        }
      } else if (!imdbNum && ctx.query.mediaType === "movie" && item.MovieKind === "episode") {
        continue;
      }

      const lang = normalizeLang(item.ISO639 || item.LanguageName || "en");
      if (!matchesPreferred(lang, ctx.prefs)) continue;

      const fmt = detectFormat(item.SubFileName || `.${item.SubFormat || "srt"}`, "srt");
      const rel = (item.MovieReleaseName || item.SubFileName || "").trim();
      const downloads = item.SubDownloadsCnt ? Number(item.SubDownloadsCnt) : undefined;

      out.push({
        id: encodeToken({
          p: "os_gz",
          u: dlLink,
          f: fmt,
          s: ctx.query.season,
          e: ctx.query.episode,
          ua: OS_REST_UA,
        }),
        lang,
        label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
        format: fmt,
        releaseName: rel || undefined,
        sourceName: "OpenSubtitles",
        downloads: Number.isFinite(downloads) ? downloads : undefined,
        hearingImpaired: item.SubHearingImpaired === "1",
      });
    }
  }

  out.sort((a, b) => (b.downloads ?? 0) - (a.downloads ?? 0));
  return out.slice(0, 45);
}

async function searchOpenSubtitlesStremioV3(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (!ctx.imdb) return [];
  const isSeries = ctx.query.mediaType === "series" && ctx.query.season && ctx.query.episode;
  const stremioType = isSeries ? "series" : "movie";
  const stremioId = isSeries ? `${ctx.imdb}:${ctx.query.season}:${ctx.query.episode}` : ctx.imdb;
  const resp = await vessel.http.get(
    `https://opensubtitles-v3.strem.io/subtitles/${stremioType}/${encodeURIComponent(stremioId)}.json`,
    { headers: { "User-Agent": UA }, timeoutMs: 10000 },
  );
  if (resp.status >= 400) return [];
  const json = resp.json<{
    subtitles?: Array<{
      id?: string;
      url?: string;
      lang?: string;
      subtitleFileName?: string;
      movieReleaseName?: string;
    }>;
  }>();

  const out: SubtitleCandidate[] = [];
  for (const sub of json.subtitles ?? []) {
    if (!sub.url) continue;
    const lang = normalizeLang(sub.lang || "en");
    if (!matchesPreferred(lang, ctx.prefs)) continue;
    const rel = (sub.movieReleaseName || sub.subtitleFileName || "").trim();
    const fmt = detectFormat(sub.subtitleFileName || sub.url, "srt");
    out.push({
      id: encodeToken({ p: "direct", u: sub.url, f: fmt }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : `${langDisplayName(lang)} · OpenSubtitles v3`,
      format: fmt,
      releaseName: rel || undefined,
      sourceName: "OpenSubtitles",
    });
  }
  return out.slice(0, 30);
}

async function searchOpenSubtitles(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("openSubtitlesApiKey");

  if (apiKey) {
    const params = new URLSearchParams();
    if (ctx.imdb) {
      params.set("imdb_id", ctx.imdb.replace(/^tt/, ""));
    } else if (ctx.query.externalIds?.tmdb) {
      params.set("tmdb_id", String(ctx.query.externalIds.tmdb));
    }
    const qText = ctx.titles[0] || "";
    if (qText && !ctx.imdb && !ctx.query.externalIds?.tmdb) params.set("query", qText);
    if (ctx.query.mediaType === "movie") params.set("type", "movie");
    if (ctx.query.mediaType === "series") {
      params.set("type", "episode");
      if (ctx.query.season) params.set("season_number", String(ctx.query.season));
      if (ctx.query.episode) params.set("episode_number", String(ctx.query.episode));
    }
    if (ctx.prefs.length) {
      params.set("languages", ctx.prefs.join(","));
    }

    try {
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
      if (resp.status < 400) {
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
            id: encodeToken({
              p: "os_api",
              u: "https://api.opensubtitles.com/api/v1/download",
              fileId: file.file_id,
              f: fmt,
              k: apiKey,
            }),
            lang,
            label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
            format: fmt,
            releaseName: rel || undefined,
            sourceName: "OpenSubtitles",
            downloads: attrs.download_count,
            hearingImpaired: Boolean(attrs.hearing_impaired),
          });
        }
        if (out.length > 0) return out;
      }
    } catch (err) {
      if (isDomainBlockedError(err)) throw err;
      // Fall through to key-free endpoints below.
    }
  }

  const [restRes, v3Res] = await Promise.allSettled([
    searchOpenSubtitlesRestLegacy(ctx),
    searchOpenSubtitlesStremioV3(ctx),
  ]);

  const combined: SubtitleCandidate[] = [];
  const seenLabels = new Set<string>();
  for (const r of [restRes, v3Res]) {
    if (r.status === "rejected") {
      if (isDomainBlockedError(r.reason)) throw r.reason;
      continue;
    }
    for (const item of r.value) {
      const key = `${item.lang}:${(item.releaseName || item.label).toLowerCase()}`;
      if (seenLabels.has(key)) continue;
      seenLabels.add(key);
      combined.push(item);
    }
  }
  return combined.slice(0, 50);
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

async function resolveOpenSubtitlesGz(tok: ResolveToken): Promise<ResolvedSubtitle> {
  const resp = await vessel.http.get(tok.u, {
    headers: { "User-Agent": tok.ua || OS_REST_UA },
    timeoutMs: 15000,
  });
  if (resp.status >= 400) {
    throw new Error(`OpenSubtitles download HTTP ${resp.status}`);
  }
  const picked = decodeArchiveOrText(resp.bytes(), tok.s, tok.e, tok.f || "srt");
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

// ---------------------------------------------------------------------------
// 2. SubDL (Official API when key set; key-free subdl.com web fallback)
// ---------------------------------------------------------------------------
function extractSubdlLangFromOffsets(
  html: string,
  rowIdx: number,
  langSections: Array<{ idx: number; lang: string }>,
): string {
  let active = "en";
  for (const sec of langSections) {
    if (sec.idx <= rowIdx) active = sec.lang;
    else break;
  }
  return active;
}

function parseSubdlHtmlRows(
  pageHtml: string,
  forcedLang: string | null,
  ctx: SearchContext,
): SubtitleCandidate[] {
  const doc = vessel.html.parse(pageHtml);
  const rows = doc.select("[data-row]");
  if (!rows.length) return [];

  const langSections: Array<{ idx: number; lang: string }> = [];
  if (!forcedLang) {
    const h2Rx = /<h2[^>]*class="[^"]*text-lg[^"]*"[^>]*>([^<]+)<\/h2>/gi;
    let m: RegExpExecArray | null;
    while ((m = h2Rx.exec(pageHtml)) !== null) {
      const code = normalizeLang(m[1]);
      if (code && code !== "und") {
        langSections.push({ idx: m.index, lang: code });
      }
    }
  }

  const out: SubtitleCandidate[] = [];
  const wantSeries = ctx.query.mediaType === "series" && ctx.query.episode;
  const ep = ctx.query.episode ?? 0;
  const epRx = episodeRegex(ctx.query.season, ctx.query.episode);
  let searchPos = 0;

  for (const row of rows) {
    const dlHref = row.selectOne("a[data-subtitle-download]")?.attr("href") || "";
    if (!dlHref) continue;

    let lang = forcedLang || "en";
    if (!forcedLang && langSections.length > 0) {
      const rowId = row.attr("data-id") || "";
      const marker = rowId ? `data-id="${rowId}"` : dlHref;
      const foundIdx = pageHtml.indexOf(marker, searchPos);
      if (foundIdx >= 0) {
        searchPos = foundIdx;
        lang = extractSubdlLangFromOffsets(pageHtml, foundIdx, langSections);
      }
    }
    if (!matchesPreferred(lang, ctx.prefs)) continue;

    const rel = (row.selectOne("a[data-subtitle-title] h4")?.text() ||
      row.selectOne("a[data-subtitle-title]")?.text() ||
      "").replace(/\s+/g, " ").trim();

    if (wantSeries) {
      const epFromRaw = row.attr("data-episode-from");
      const epToRaw = row.attr("data-episode-to");
      const epFrom = epFromRaw ? Number(epFromRaw) : 0;
      const epTo = epToRaw ? Number(epToRaw) : epFrom;
      const isFullSeason =
        row.attr("data-full-season") !== null && epFrom === 0 && !epToRaw;
      const epRangeMatches = epFrom > 0 && ep >= epFrom && ep <= epTo;
      const textMatches = epRx ? epRx.test(rel) : false;
      const packMatches = /\b(season|complete|full|pack)\b/i.test(rel) && epFrom === 0;
      if (!epRangeMatches && !textMatches && !isFullSeason && !packMatches) {
        continue;
      }
    }

    const dlUrl = dlHref.startsWith("http")
      ? dlHref
      : `https://dl.subdl.com${dlHref.startsWith("/") ? "" : "/"}${dlHref}`;
    const downloadsAttr = row.attr("data-downloads");
    const downloads = downloadsAttr ? Number(downloadsAttr) : undefined;
    const hi = row.attr("data-hi") !== null;

    out.push({
      id: encodeToken({
        p: "zip",
        u: dlUrl,
        s: ctx.query.season,
        e: ctx.query.episode,
        r: "https://subdl.com/",
      }),
      lang,
      label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
      format: "srt",
      releaseName: rel || undefined,
      sourceName: "SubDL",
      downloads: Number.isFinite(downloads) ? downloads : undefined,
      hearingImpaired: hi,
    });
    if (out.length >= 30) break;
  }

  return out;
}

async function searchSubdlWeb(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  const wantSeries = ctx.query.mediaType === "series";
  let showHref = "";

  for (const term of ctx.titles) {
    const sResp = await vessel.http.get(
      `https://subdl.com/search/${encodeURIComponent(term)}`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (sResp.status >= 400) continue;
    const sDoc = vessel.html.parse(sResp.text());
    const cards = sDoc.select('[data-pc-feed] a[href^="/subtitle/sd"]');

    let bestHref = "";
    let bestScore = 0;
    for (const card of cards) {
      const href = card.attr("href") || "";
      if (!href) continue;
      const cardText = card.text().replace(/\s+/g, " ").trim();
      const titleText = card.selectOne("h3")?.text().trim() || cardText;
      const isTvCard = /\btv\b/i.test(cardText) && !/\bmovie\b/i.test(cardText);
      const isMovieCard = /\bmovie\b/i.test(cardText);
      if (wantSeries && isMovieCard) continue;
      if (!wantSeries && isTvCard) continue;

      let score = titleMatchScore(titleText, ctx.titles);
      if (score <= 0) continue;
      if (ctx.year) {
        const yrM = /\((\d{4})\)/.exec(titleText);
        if (yrM) {
          const y = Number(yrM[1]);
          if (Math.abs(ctx.year - y) <= 1) score += 30;
          else score -= 30;
        }
      }
      if (score > bestScore) {
        bestScore = score;
        bestHref = href;
      }
    }

    if (bestHref && bestScore >= 40) {
      showHref = bestHref;
      break;
    }
  }

  if (!showHref) return [];

  let targetBaseHref = showHref;
  if (wantSeries && ctx.query.season) {
    const mainResp = await vessel.http.get(`https://subdl.com${showHref}`, {
      headers: { "User-Agent": UA, Referer: "https://subdl.com/" },
      timeoutMs: 12000,
    });
    if (mainResp.status >= 400) return [];
    const mainDoc = vessel.html.parse(mainResp.text());
    const seasonRx = new RegExp(`\\bseason\\s*0*${ctx.query.season}\\b`, "i");
    let matchedSeasonHref = "";
    for (const a of mainDoc.select(`a[href^="${showHref}/"]`)) {
      const h = a.attr("href") || "";
      const txt = a.text().replace(/\s+/g, " ").trim();
      if (seasonRx.test(txt)) {
        matchedSeasonHref = h;
        break;
      }
    }
    if (!matchedSeasonHref) return [];
    targetBaseHref = matchedSeasonHref;
  }

  const langSlugs: Array<{ lang: string; slug: string }> = [];
  if (ctx.prefs.length > 0) {
    for (const p of ctx.prefs.slice(0, 3)) {
      const slug = CODE_TO_SUBDL_SLUG[p];
      if (slug && !langSlugs.some((x) => x.slug === slug)) {
        langSlugs.push({ lang: p, slug });
      }
    }
  }

  if (langSlugs.length > 0) {
    const pages = await Promise.allSettled(
      langSlugs.map(async ({ lang, slug }) => {
        const resp = await vessel.http.get(`https://subdl.com${targetBaseHref}/${slug}`, {
          headers: { "User-Agent": UA, Referer: `https://subdl.com${targetBaseHref}` },
          timeoutMs: 12000,
        });
        if (resp.status >= 400) return [];
        return parseSubdlHtmlRows(resp.text(), lang, ctx);
      }),
    );
    const out: SubtitleCandidate[] = [];
    for (const p of pages) {
      if (p.status === "rejected") {
        if (isDomainBlockedError(p.reason)) throw p.reason;
        continue;
      }
      out.push(...p.value);
    }
    return out;
  }

  const pageResp = await vessel.http.get(`https://subdl.com${targetBaseHref}`, {
    headers: { "User-Agent": UA, Referer: "https://subdl.com/" },
    timeoutMs: 12000,
  });
  if (pageResp.status >= 400) return [];
  return parseSubdlHtmlRows(pageResp.text(), null, ctx);
}

async function searchSubdl(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("subdlApiKey");
  if (apiKey) {
    const params = new URLSearchParams();
    params.set("api_key", apiKey);
    if (ctx.imdb) {
      params.set("imdb_id", ctx.imdb);
    } else if (ctx.query.externalIds?.tmdb) {
      params.set("tmdb_id", String(ctx.query.externalIds.tmdb));
    } else {
      params.set("film_name", ctx.titles[0] || ctx.query.title);
      if (ctx.year) params.set("year", String(ctx.year));
    }
    params.set("type", ctx.query.mediaType === "series" ? "tv" : "movie");
    if (ctx.query.mediaType === "series") {
      if (ctx.query.season) params.set("season_number", String(ctx.query.season));
      if (ctx.query.episode) params.set("episode_number", String(ctx.query.episode));
    }
    if (ctx.prefs.length) {
      params.set("languages", ctx.prefs.map((c) => c.toUpperCase()).join(","));
    }
    params.set("subs_per_page", "30");

    try {
      const resp = await vessel.http.get(`https://api.subdl.com/api/v1/subtitles?${params.toString()}`, {
        headers: { "User-Agent": UA, Accept: "application/json" },
        timeoutMs: 12000,
      });
      if (resp.status < 400) {
        const data = resp.json<{
          status?: boolean;
          subtitles?: Array<{
            release_name?: string;
            name?: string;
            lang?: string;
            url?: string;
            hi?: boolean;
          }>;
        }>();
        if (data.status !== false && (data.subtitles?.length ?? 0) > 0) {
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
                s: ctx.query.season,
                e: ctx.query.episode,
              }),
              lang,
              label: rel ? `${langDisplayName(lang)} · ${rel}` : langDisplayName(lang),
              format: "srt",
              releaseName: rel || undefined,
              sourceName: "SubDL",
              hearingImpaired: Boolean(item.hi),
            });
          }
          if (out.length > 0) return out;
        }
      }
    } catch (err) {
      if (isDomainBlockedError(err)) throw err;
    }
  }

  return searchSubdlWeb(ctx);
}

// ---------------------------------------------------------------------------
// 3. SubSource (API key required)
// ---------------------------------------------------------------------------
async function searchSubsource(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  const apiKey = await readStrSetting("subsourceApiKey");
  if (!apiKey) {
    return [];
  }
  const headers = {
    "X-API-Key": apiKey,
    "User-Agent": UA,
    Accept: "application/json",
  };
  const searchParams = new URLSearchParams();
  if (ctx.imdb) {
    searchParams.set("searchType", "imdb");
    searchParams.set("imdb", ctx.imdb);
  } else {
    searchParams.set("searchType", "text");
    searchParams.set("q", ctx.titles[0] || ctx.query.title);
    if (ctx.year) searchParams.set("year", String(ctx.year));
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
  if (ctx.prefs.length) subParams.set("language", ctx.prefs.join(","));
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
    if (!matchesPreferred(lang, ctx.prefs)) continue;
    const rel = (item.releaseInfo ?? []).join(" ").trim();
    out.push({
      id: encodeToken({
        p: "subsource",
        u: `https://api.subsource.net/api/v1/subtitles/${sid}/download`,
        s: ctx.query.season,
        e: ctx.query.episode,
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
function detectTurkceAltyaziLang(row: {
  selectOne(sel: string): { attr(name: string): string | null; text(): string } | null;
}): string {
  const flagEl = row.selectOne(".aldil span, .aldil img");
  const cls = (flagEl?.attr("class") || "").toLowerCase();
  const hint = (
    flagEl?.attr("title") ||
    flagEl?.attr("alt") ||
    row.selectOne(".aldil")?.text() ||
    ""
  )
    .trim()
    .toLowerCase();

  if (cls.includes("flagen") || cls.includes("flageng") || hint.includes("eng") || hint.includes("ing")) {
    return "en";
  }
  if (cls.includes("flagtr") || hint.includes("tur") || hint.includes("türk")) {
    return "tr";
  }
  return normalizeLang(hint || "tr");
}

async function searchTurkceAltyazi(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (ctx.prefs.length > 0 && !ctx.prefs.includes("tr") && !ctx.prefs.includes("en")) {
    return [];
  }

  let movUrl = "";
  let movHtml = "";

  // 1. Direct IMDb URL lookup when IMDb ID is known (avoids picking unrelated series).
  if (ctx.imdb) {
    const num = ctx.imdb.replace(/^tt/, "");
    const directUrl = `https://turkcealtyazi.org/mov/${num}/`;
    const directResp = await vessel.http.get(directUrl, {
      headers: { "User-Agent": UA, Referer: "https://turkcealtyazi.org/" },
      timeoutMs: 12000,
    });
    if (
      directResp.status < 400 &&
      !directResp.url.includes("404-error") &&
      directResp.url.includes(`/${num}/`)
    ) {
      movUrl = directResp.url;
      movHtml = directResp.text();
    }
  }

  // 2. Fallback to find.php ONLY inspecting actual search result links inside `.sub-container .nblock`.
  if (!movHtml) {
    const num = ctx.imdb ? ctx.imdb.replace(/^tt/, "") : null;
    for (const term of ctx.titles) {
      const findResp = await vessel.http.get(
        `https://turkcealtyazi.org/find.php?cat=sub&find=${encodeURIComponent(term)}`,
        { headers: { "User-Agent": UA }, timeoutMs: 12000 },
      );
      if (findResp.status >= 400) continue;

      // Redirected straight to a movie/series page.
      if (findResp.url.includes("/mov/") && !findResp.url.includes("404-error")) {
        movUrl = findResp.url;
        movHtml = findResp.text();
        break;
      }

      const doc = vessel.html.parse(findResp.text());
      // CRITICAL: Scope strictly to `.sub-container .nblock` so sidebar top-lists (`.toptabsdiv`) are never matched!
      const resultLinks = doc.select('.sub-container .nblock a[href^="/mov/"]');
      let bestHref = "";
      let bestScore = 0;

      for (const a of resultLinks) {
        const href = a.attr("href") || "";
        if (!href || href.includes("404-error")) continue;
        if (num && href.includes(`/${num}/`)) {
          bestHref = href;
          bestScore = 1000;
          break;
        }
        const candTitle = (a.attr("title") || a.text() || "").replace(/\s+/g, " ").trim();
        if (!candTitle) continue;
        const score = titleMatchScore(candTitle, ctx.titles);
        if (score > bestScore) {
          bestScore = score;
          bestHref = href;
        }
      }

      if (bestHref && bestScore >= 60) {
        movUrl = `https://turkcealtyazi.org${bestHref}`;
        break;
      }
    }
  }

  if (!movUrl) return [];
  if (!movHtml) {
    const movResp = await vessel.http.get(movUrl, {
      headers: { "User-Agent": UA, Referer: "https://turkcealtyazi.org/" },
      timeoutMs: 12000,
    });
    if (movResp.status >= 400 || movResp.url.includes("404-error")) return [];
    movHtml = movResp.text();
  }

  const movDoc = vessel.html.parse(movHtml);
  const rows = movDoc.select(".altyazi-list-wrapper > div > div[class*='altsonsez']");
  const epRx = episodeRegex(ctx.query.season, ctx.query.episode);
  const exactMatches: SubtitleCandidate[] = [];
  const packMatches: SubtitleCandidate[] = [];

  for (const row of rows) {
    const link = row.selectOne('a[href^="/sub/"]');
    const href = link?.attr("href");
    if (!href) continue;

    const lang = detectTurkceAltyaziLang(row);
    if (!matchesPreferred(lang, ctx.prefs)) continue;

    const alcdText = (row.selectOne(".alcd")?.text() || "").replace(/\s+/g, " ").trim();
    const rowText = `${alcdText} ${row.text()}`.replace(/\s+/g, " ").trim();
    let isPack = false;

    if (ctx.query.mediaType === "series" && ctx.query.season && ctx.query.episode) {
      const isExactEp = epRx ? epRx.test(rowText) : false;
      const seasonTagRx = new RegExp(`\\b(?:s0*${ctx.query.season}|sezon\\s*0*${ctx.query.season})\\b`, "i");
      const hasPackWord = /\b(?:paket|pack|complete|tüm|full)\b/i.test(alcdText || rowText);
      const hasEpisodeNumber = /\b(?:e\d+|b\d+|\d+x\d+)\b/i.test(alcdText);
      const isSeasonPack = seasonTagRx.test(alcdText || rowText) && (hasPackWord || !hasEpisodeNumber);
      if (!isExactEp && !isSeasonPack) continue;
      isPack = !isExactEp && isSeasonPack;
    }

    const ripEl = row.selectOne(".alrip, .altyazi-rip");
    const translator = row.selectOne(".alcevirmen")?.text().trim() || "";
    const detailPart = [alcdText, ripEl?.text().trim() || "", translator]
      .filter((s) => s.length > 0 && s !== "Film" && s !== "Dizi")
      .join(" · ");
    const rel = detailPart || link?.attr("title")?.trim() || "TurkceAltyazi";
    const subUrl = `https://turkcealtyazi.org${href}`;

    const candidate: SubtitleCandidate = {
      id: encodeToken({
        p: "ta",
        u: subUrl,
        s: ctx.query.season,
        e: ctx.query.episode,
      }),
      lang,
      label: `${langDisplayName(lang)} · ${rel}`,
      format: "srt",
      releaseName: rel,
      sourceName: "TurkceAltyazi",
    };

    if (isPack) packMatches.push(candidate);
    else exactMatches.push(candidate);
  }

  return [...exactMatches, ...packMatches].slice(0, 30);
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
  const picked = decodeArchiveOrText(dlResp.bytes(), tok.s, tok.e, "srt");
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

// ---------------------------------------------------------------------------
// 5. YIFY Subtitles (Movies, API key-free)
// ---------------------------------------------------------------------------
async function searchYify(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (ctx.query.mediaType !== "movie") return [];
  let movieUrl = ctx.imdb ? `https://yifysubtitles.ch/movie-imdb/${ctx.imdb}` : "";

  if (!movieUrl) {
    for (const term of ctx.titles) {
      const sResp = await vessel.http.get(
        `https://yts-subs.com/search/${encodeURIComponent(term)}`,
        { headers: { "User-Agent": UA }, timeoutMs: 12000 },
      );
      if (sResp.status >= 400) continue;
      const sDoc = vessel.html.parse(sResp.text());
      const hit = sDoc.selectOne('a[href^="/movie-imdb/"]')?.attr("href");
      if (hit) {
        movieUrl = `https://yifysubtitles.ch${hit}`;
        break;
      }
    }
  }
  if (!movieUrl) return [];

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
    if (!matchesPreferred(lang, ctx.prefs)) continue;

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
async function searchAddic7ed(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (ctx.query.mediaType !== "series" || !ctx.query.season || !ctx.query.episode) return [];
  if (!ctx.titles.length) return [];

  let epUrl = "";
  const seasonEpRx = new RegExp(`/${ctx.query.season}/${ctx.query.episode}/`, "i");

  for (const term of ctx.titles) {
    const srchResp = await vessel.http.get(
      `https://www.addic7ed.com/srch.php?search=${encodeURIComponent(term)}&Submit=Search`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (srchResp.status >= 400) continue;

    const srchDoc = vessel.html.parse(srchResp.text());
    const serieLinks = srchDoc.select('a[href*="serie/"]');
    for (const a of serieLinks) {
      const href = a.attr("href") || "";
      const m = /serie\/([^/]+)\//i.exec(href);
      const showSlugName = m ? decodeURIComponent(m[1]).replace(/_/g, " ") : a.text();
      if (titleMatchScore(showSlugName, ctx.titles) < 55) continue;

      if (seasonEpRx.test(href)) {
        epUrl = href.startsWith("http") ? href : `https://www.addic7ed.com/${href.replace(/^\/+/, "")}`;
        break;
      }
      if (!epUrl && m) {
        epUrl = `https://www.addic7ed.com/serie/${m[1]}/${ctx.query.season}/${ctx.query.episode}/1`;
      }
    }
    if (epUrl) break;
  }
  if (!epUrl) return [];

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
    if (!matchesPreferred(lang, ctx.prefs)) continue;

    // Addic7ed HTML has duplicate `class="face-button" class="buttonDownload"` which drops `buttonDownload` in HTML5 DOM.
    const dl = block.selectOne('a[href*="updated/"], a[href*="original/"]');
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
async function searchTvSubtitles(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (ctx.query.mediaType !== "series" || !ctx.query.season || !ctx.query.episode) return [];
  if (!ctx.titles.length) return [];

  let showHref = "";
  for (const term of ctx.titles) {
    const sResp = await vessel.http.post(
      "https://www.tvsubtitles.net/search2.php",
      `qs=${encodeURIComponent(term)}`,
      {
        headers: {
          "User-Agent": UA,
          Referer: "https://www.tvsubtitles.net/",
          "Content-Type": "application/x-www-form-urlencoded",
        },
        timeoutMs: 12000,
      },
    );
    if (sResp.status >= 400) continue;

    const sDoc = vessel.html.parse(sResp.text());
    let bestHref = "";
    let bestScore = 0;
    for (const a of sDoc.select('a[href^="/tvshow-"]')) {
      const href = a.attr("href") || "";
      if (!/^\/tvshow-\d+\.html$/.test(href)) continue;
      const score = titleMatchScore(a.text(), ctx.titles);
      if (score > bestScore) {
        bestScore = score;
        bestHref = href;
      }
    }
    if (bestHref && bestScore >= 60) {
      showHref = bestHref;
      break;
    }
  }
  if (!showHref) return [];

  const showIdMatch = /\/tvshow-(\d+)\.html/.exec(showHref);
  if (!showIdMatch) return [];
  const seasonUrl = `https://www.tvsubtitles.net/tvshow-${showIdMatch[1]}-${ctx.query.season}.html`;
  const seasonResp = await vessel.http.get(seasonUrl, {
    headers: { "User-Agent": UA, Referer: "https://www.tvsubtitles.net/" },
    timeoutMs: 12000,
  });
  if (seasonResp.status >= 400) return [];

  const seasonDoc = vessel.html.parse(seasonResp.text());
  const targetEp = `${ctx.query.season}x${pad2(ctx.query.episode)}`;
  let epHref = "";
  for (const row of seasonDoc.select("table tr")) {
    const text = row.text();
    if (text.includes(targetEp) || text.includes(`${ctx.query.season}x${ctx.query.episode}`)) {
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
    if (!matchesPreferred(lang, ctx.prefs)) continue;

    const rel = a.text().replace(/\s+/g, " ").trim();
    out.push({
      id: encodeToken({
        p: "tvsub",
        u: `https://www.tvsubtitles.net/download-${subMatch[1]}.html`,
        r: `https://www.tvsubtitles.net/subtitle-${subMatch[1]}.html`,
        s: ctx.query.season,
        e: ctx.query.episode,
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
  const picked = decodeArchiveOrText(zipResp.bytes(), tok.s, tok.e, "srt");
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

// ---------------------------------------------------------------------------
// 8. Kitsunekko (Anime, API key-free)
// ---------------------------------------------------------------------------
async function searchKitsunekko(ctx: SearchContext): Promise<SubtitleCandidate[]> {
  if (ctx.prefs.length > 0 && !ctx.prefs.includes("ja") && !ctx.prefs.includes("en")) {
    return [];
  }
  if (!ctx.titles.length) return [];

  const buckets: Array<{ lang: string; dir: string }> = [];
  if (!ctx.prefs.length || ctx.prefs.includes("ja")) {
    buckets.push({ lang: "ja", dir: "subtitles%2Fjapanese%2F" });
  }
  if (!ctx.prefs.length || ctx.prefs.includes("en")) {
    buckets.push({ lang: "en", dir: "subtitles%2Fenglish%2F" });
  }

  const out: SubtitleCandidate[] = [];
  const epNum = ctx.query.episode;

  for (const bucket of buckets) {
    const dirResp = await vessel.http.get(
      `https://kitsunekko.net/dirlist.php?dir=${bucket.dir}`,
      { headers: { "User-Agent": UA }, timeoutMs: 12000 },
    );
    if (dirResp.status >= 400) continue;

    const doc = vessel.html.parse(dirResp.text());
    let matchedHref = "";
    let bestScore = 0;
    for (const a of doc.select('a[href*="dirlist.php?dir="]')) {
      const href = a.attr("href") || "";
      if (href.endsWith(bucket.dir) || href.endsWith(`${bucket.dir.replace(/%2F/g, "/")}`)) continue;
      const score = titleMatchScore(a.text(), ctx.titles);
      if (score > bestScore) {
        bestScore = score;
        matchedHref = href;
      }
    }
    if (!matchedHref || bestScore < 60) continue;

    const showUrl = matchedHref.startsWith("http")
      ? matchedHref
      : `https://kitsunekko.net/${matchedHref.replace(/^\/+/, "")}`;
    const showResp = await vessel.http.get(showUrl, {
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
          s: ctx.query.season,
          e: ctx.query.episode,
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
  const picked = decodeArchiveOrText(resp.bytes(), tok.s, tok.e, tok.f || detectFormat(tok.u, "srt"));
  return {
    format: picked.format,
    source: { kind: "inline_content", content: picked.content },
  };
}

async function resolveZip(tok: ResolveToken, extraHeaders?: Record<string, string>): Promise<ResolvedSubtitle> {
  const headers: Record<string, string> = { "User-Agent": UA, ...(extraHeaders ?? {}) };
  if (tok.r) headers["Referer"] = tok.r;

  const candidates = [tok.u];
  if (tok.u.startsWith("https://dl.subdl.com/")) {
    candidates.push(tok.u.replace("https://dl.subdl.com/", "https://api3.subdl.com/"));
  }

  let lastErr: unknown = null;
  for (const url of candidates) {
    try {
      const resp = await vessel.http.get(url, { headers, timeoutMs: 15000 });
      if (resp.status >= 400) {
        lastErr = new Error(`Subtitle archive HTTP ${resp.status}`);
        continue;
      }
      const picked = decodeArchiveOrText(resp.bytes(), tok.s, tok.e, tok.f || "srt");
      return {
        format: picked.format,
        source: { kind: "inline_content", content: picked.content },
      };
    } catch (err) {
      if (isDomainBlockedError(err)) throw err;
      lastErr = err;
    }
  }
  throw lastErr instanceof Error ? lastErr : new Error(String(lastErr || "Subtitle archive failed"));
}

export default vessel.definePlugin({
  async searchSubtitles(query: SubtitleQuery): Promise<SubtitleCandidate[]> {
    const prefs = preferredSet(query);
    const ctx = await buildSearchContext(query, prefs);

    const tasks: Array<{ name: string; run: () => Promise<SubtitleCandidate[]> }> = [];

    if (await readBoolSetting("enableOpenSubtitles", true)) {
      tasks.push({ name: "OpenSubtitles", run: () => searchOpenSubtitles(ctx) });
    }
    if (await readBoolSetting("enableSubdl", true)) {
      tasks.push({ name: "SubDL", run: () => searchSubdl(ctx) });
    }
    if (await readBoolSetting("enableSubsource", true)) {
      tasks.push({ name: "SubSource", run: () => searchSubsource(ctx) });
    }
    if (await readBoolSetting("enableTurkceAltyazi", true)) {
      tasks.push({ name: "TurkceAltyazi", run: () => searchTurkceAltyazi(ctx) });
    }
    if (await readBoolSetting("enableYify", true)) {
      tasks.push({ name: "YIFY", run: () => searchYify(ctx) });
    }
    if (await readBoolSetting("enableAddic7ed", true)) {
      tasks.push({ name: "Addic7ed", run: () => searchAddic7ed(ctx) });
    }
    if (await readBoolSetting("enableTvSubtitles", true)) {
      tasks.push({ name: "TVSubtitles", run: () => searchTvSubtitles(ctx) });
    }
    if (await readBoolSetting("enableKitsunekko", true)) {
      tasks.push({ name: "Kitsunekko", run: () => searchKitsunekko(ctx) });
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
      case "os_gz":
        return resolveOpenSubtitlesGz(tok);
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
