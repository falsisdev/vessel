//! External ecosystem bridges (`Nuvio`, `Stremio`, `CloudStream` detection) and open-source auditing.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use url::Url;
use vessel_domain::{
    ContentCategory, PluginCapability, PluginId, PluginManifest, RepoIndex, RepoPluginEntry,
    VesselError, VesselResult,
};

use crate::module_loader::PluginBundle;
use crate::package::LoadedPackage;
use crate::security::is_forbidden_ip;
use crate::ts_strip::sha256_hex;

pub const NUVIO_SCHEME_PREFIX: &str = "nuvio-plugin://";

#[derive(Debug, Clone)]
pub enum ExternalManifestKind {
    NuvioRepo(Box<RepoIndex>),
    StremioAddon(Box<LoadedPackage>),
}

/// Normalizes external IDs (`m3u.anthology.addon`, etc.) into filesystem-safe `PluginId`s.
pub fn sanitize_plugin_id(raw: &str) -> VesselResult<PluginId> {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    let clean = out.trim_matches(['.', '-', '_']).to_owned();
    PluginId::new(if clean.is_empty() { "plugin" } else { &clean })
}

/// Ensures remote JS/TS plugin source is readable, open-source UTF-8 text rather than hidden/obfuscated/binary code.
pub fn verify_open_source_js(code: &str, origin: &str) -> VesselResult<()> {
    if code.trim().is_empty() {
        return Err(VesselError::InvalidInput(format!(
            "Plugin source at '{origin}' is empty and cannot be audited."
        )));
    }
    if code.contains('\0')
        || code.starts_with("dex\n")
        || code.starts_with("PK\x03\x04")
        || code.starts_with("\x7fELF")
    {
        return Err(VesselError::InvalidInput(format!(
            "Plugin source at '{origin}' is a compiled binary rather than open-source code. Vessel refused to add it."
        )));
    }

    // Detect `javascript-obfuscator` / unreadable hidden hex-array payloads (`_0x1a2b(...)`).
    let mut hex_ident_hits = 0usize;
    let bytes = code.as_bytes();
    let mut i = 0usize;
    while i + 5 < bytes.len() {
        if bytes[i] == b'_'
            && bytes[i + 1] == b'0'
            && bytes[i + 2] == b'x'
            && bytes[i + 3..i + 6].iter().all(|b| b.is_ascii_hexdigit())
        {
            hex_ident_hits += 1;
            if hex_ident_hits >= 16 {
                return Err(VesselError::InvalidInput(format!(
                    "Plugin source at '{origin}' uses heavy obfuscation (_0x hex identifiers) instead of readable open-source code. Vessel refused to add it."
                )));
            }
            i += 6;
            continue;
        }
        i += 1;
    }

    // Reject single-blob eval(atob(...)) / Function(atob(...)) payloads hiding all source code.
    let compact_len = code.chars().filter(|c| !c.is_whitespace()).count();
    let has_readable_defs = code.contains("function ")
        || code.contains("getStreams")
        || code.contains("getCatalog")
        || code.contains("getMeta")
        || code.contains("export ");
    if (code.contains("eval(atob(") || code.contains("Function(atob("))
        && (!has_readable_defs || compact_len < 600)
    {
        return Err(VesselError::InvalidInput(format!(
            "Plugin source at '{origin}' hides its logic behind encoded eval(atob(...)). Vessel refused to add it."
        )));
    }

    Ok(())
}

/// Scans open-source code for public `https://` origin hostnames so manifest declarations match code usage.
pub fn extract_literal_https_domains(code: &str, seed_urls: &[&str]) -> Vec<String> {
    let mut domains = BTreeSet::new();
    for raw in seed_urls {
        if let Ok(u) = Url::parse(raw)
            && let Some(host) = u.host_str()
            && is_public_domain_candidate(host)
        {
            domains.insert(host.to_ascii_lowercase());
        }
    }

    let mut search_from = 0usize;
    while let Some(pos) = code[search_from..].find("https://") {
        let start = search_from + pos;
        let rest = &code[start..];
        let end = rest
            .find(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '"' | '\'' | '`' | ')' | ']' | '}' | '<' | '>' | ',' | ';' | '\\'
                    )
            })
            .unwrap_or(rest.len());
        let candidate = &rest[..end];
        if let Ok(parsed) = Url::parse(candidate)
            && let Some(host) = parsed.host_str()
            && is_public_domain_candidate(host)
        {
            domains.insert(host.to_ascii_lowercase());
        }
        search_from = start + 8;
    }

    domains.into_iter().collect()
}

fn is_public_domain_candidate(host: &str) -> bool {
    let h = host.trim();
    if h.is_empty() || h.eq_ignore_ascii_case("localhost") || !h.contains('.') {
        return false;
    }
    if let Ok(ip) = h
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<std::net::IpAddr>()
    {
        return !is_forbidden_ip(ip);
    }
    true
}

fn push_unique<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if !list.contains(&item) {
        list.push(item);
    }
}

fn map_content_types(raw_types: Option<&Vec<Value>>) -> Vec<ContentCategory> {
    let mut cats = Vec::new();
    if let Some(items) = raw_types {
        for item in items {
            let Some(s) = item.as_str() else { continue };
            match s.to_ascii_lowercase().as_str() {
                "movie" => push_unique(&mut cats, ContentCategory::Movie),
                "tv" | "series" => push_unique(&mut cats, ContentCategory::Series),
                "anime" => push_unique(&mut cats, ContentCategory::Anime),
                "channel" | "live" | "livetv" => push_unique(&mut cats, ContentCategory::Live),
                _ => {}
            }
        }
    }
    if cats.is_empty() {
        cats.push(ContentCategory::Series);
        cats.push(ContentCategory::Movie);
    }
    cats
}

fn map_capabilities(
    resources: Option<&Vec<Value>>,
    content_types: &[ContentCategory],
) -> Vec<PluginCapability> {
    let mut caps = vec![PluginCapability::Vod, PluginCapability::Search];
    if content_types.contains(&ContentCategory::Live) {
        push_unique(&mut caps, PluginCapability::LiveTv);
    }
    if let Some(res) = resources {
        for entry in res {
            let name = entry
                .as_str()
                .or_else(|| entry.get("name").and_then(Value::as_str));
            if matches!(name, Some("subtitles")) {
                push_unique(&mut caps, PluginCapability::Subtitles);
            }
        }
    }
    caps
}

fn map_languages(val: &Value) -> Vec<String> {
    let mut langs = Vec::new();
    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                langs.push(s.to_owned());
            }
        }
    }
    if langs.is_empty() {
        langs.push("en".into());
    }
    langs
}

/// Checks whether `bytes` is a closed CloudStream compiled plugin list (`.cs3`) and returns a clear warning.
pub fn check_cloudstream_closed_plugins(bytes: &[u8], origin: &str) -> VesselResult<()> {
    let Ok(val) = serde_json::from_slice::<Value>(bytes) else {
        return Ok(());
    };
    let Some(arr) = val.as_array() else {
        return Ok(());
    };
    let cs3_count = arr
        .iter()
        .filter(|entry| {
            entry
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|u| u.to_ascii_lowercase().ends_with(".cs3"))
        })
        .count();

    if cs3_count > 0 {
        return Err(VesselError::InvalidInput(format!(
            "CloudStream repository '{origin}' distributes {cs3_count} compiled '.cs3' binary package(s) without readable open-source JS/TS code. Vessel only installs open-source, auditable plugins and refused to add this repository."
        )));
    }
    Ok(())
}

/// Parses either a Nuvio repository (`scrapers: [...]`) or a Stremio addon manifest into Vessel models.
pub fn parse_external_manifest(
    bytes: &[u8],
    manifest_url: &str,
) -> VesselResult<Option<ExternalManifestKind>> {
    let Ok(val) = serde_json::from_slice::<Value>(bytes) else {
        return Ok(None);
    };
    let Some(obj) = val.as_object() else {
        return Ok(None);
    };

    // 1. Nuvio Repository manifest (`{ name, version, scrapers: [...] }`)
    if let Some(scrapers) = obj.get("scrapers").and_then(Value::as_array) {
        let repo_name = obj
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Nuvio Repository")
            .to_owned();
        let repo_id = repo_name
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_owned();

        let mut plugins = Vec::with_capacity(scrapers.len());
        for sc in scrapers {
            let Some(raw_id) = sc.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(_filename) = sc.get("filename").and_then(Value::as_str) else {
                continue;
            };
            let plugin_id = sanitize_plugin_id(raw_id)?;
            let name = sc
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(raw_id)
                .to_owned();
            let version = sc
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("1.0.0")
                .to_owned();
            let enabled_in_manifest = sc.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            let raw_desc = sc.get("description").and_then(Value::as_str);
            let description = match (raw_desc, enabled_in_manifest) {
                (Some(d), true) => Some(d.to_owned()),
                (Some(d), false) => Some(format!("[Disabled in repo] {d}")),
                (None, false) => Some("[Disabled in repo]".to_owned()),
                (None, true) => None,
            };

            let content_types =
                map_content_types(sc.get("supportedTypes").and_then(Value::as_array));
            let capabilities = map_capabilities(
                sc.get("resources").and_then(Value::as_array),
                &content_types,
            );
            let languages = sc
                .get("contentLanguage")
                .map(map_languages)
                .unwrap_or_else(|| vec!["en".into()]);

            plugins.push(RepoPluginEntry {
                id: plugin_id,
                name,
                version,
                description,
                capabilities,
                content_types,
                languages,
                is_adult: sc.get("isAdult").and_then(Value::as_bool).unwrap_or(false),
                icon_url: sc.get("logo").and_then(Value::as_str).map(str::to_owned),
                download_url: None,
                source_path: Some(format!("{NUVIO_SCHEME_PREFIX}{manifest_url}#{raw_id}")),
                sha256: None,
                min_core_version: None,
            });
        }

        if plugins.is_empty() {
            return Err(VesselError::InvalidInput(format!(
                "Nuvio repository at '{manifest_url}' declared zero inspectable scrapers."
            )));
        }

        let repo_icon = obj
            .get("iconUrl")
            .or_else(|| obj.get("icon"))
            .or_else(|| obj.get("logo"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .or_else(|| plugins.iter().find_map(|p| p.icon_url.clone()));

        return Ok(Some(ExternalManifestKind::NuvioRepo(Box::new(RepoIndex {
            schema_version: 1,
            id: if repo_id.is_empty() {
                "nuvio-repo".into()
            } else {
                repo_id
            },
            name: repo_name,
            description: obj
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned),
            author: obj.get("author").and_then(Value::as_str).map(str::to_owned),
            website: None,
            icon_url: repo_icon,
            plugin_lists: Vec::new(),
            shared_extractors: None,
            plugins,
        }))));
    }

    // 2. Stremio Addon manifest (`{ id, name, version, resources, types, catalogs }`)
    if obj.get("resources").is_some()
        && obj.get("types").is_some()
        && obj.get("id").is_some()
        && obj.get("entry").is_none()
    {
        if obj
            .get("behaviorHints")
            .and_then(|h| h.get("configurationRequired"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            return Err(VesselError::InvalidInput(format!(
                "Stremio addon at '{manifest_url}' requires private external configuration and cannot be imported cleanly without hidden credentials."
            )));
        }

        let pkg = build_stremio_addon_package(&val, manifest_url)?;
        return Ok(Some(ExternalManifestKind::StremioAddon(Box::new(pkg))));
    }

    Ok(None)
}

/// Packages an audited open-source Nuvio scraper JS module into a runnable Vessel `LoadedPackage`.
pub fn build_nuvio_scraper_package(
    manifest_val: &Value,
    manifest_url: &str,
    scraper_id: &str,
    raw_js: &str,
    resolved_js_url: &str,
) -> VesselResult<LoadedPackage> {
    verify_open_source_js(raw_js, resolved_js_url)?;

    let scrapers = manifest_val
        .get("scrapers")
        .and_then(Value::as_array)
        .ok_or_else(|| VesselError::InvalidInput("manifest missing scrapers array".into()))?;
    let sc = scrapers
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(scraper_id))
        .ok_or_else(|| {
            VesselError::NotFound(format!(
                "scraper '{scraper_id}' not found in Nuvio manifest '{manifest_url}'"
            ))
        })?;

    let plugin_id = sanitize_plugin_id(scraper_id)?;
    let name = sc
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(scraper_id)
        .to_owned();
    let version = sc
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("1.0.0")
        .to_owned();
    let content_types = map_content_types(sc.get("supportedTypes").and_then(Value::as_array));
    let capabilities = map_capabilities(
        sc.get("resources").and_then(Value::as_array),
        &content_types,
    );
    let languages = sc
        .get("contentLanguage")
        .map(map_languages)
        .unwrap_or_else(|| vec!["en".into()]);

    let domains = extract_literal_https_domains(raw_js, &[manifest_url, resolved_js_url]);

    let manifest = PluginManifest {
        schema_version: 1,
        id: plugin_id,
        name,
        version,
        author: sc
            .get("author")
            .and_then(Value::as_str)
            .or_else(|| manifest_val.get("author").and_then(Value::as_str))
            .map(str::to_owned),
        description: sc
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        entry: "index.js".into(),
        icon: sc.get("logo").and_then(Value::as_str).map(str::to_owned),
        min_core_version: None,
        capabilities,
        content_types,
        languages,
        domains,
        allow_any_https: false,
        is_adult: sc.get("isAdult").and_then(Value::as_bool).unwrap_or(false),
        files: vec!["provider_shim.js".into()],
        settings_schema: Vec::new(),
    };
    manifest.validate()?;

    let meta_json = serde_json::json!({
        "id": scraper_id,
        "catalogs": sc.get("catalogs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "supportedTypes": sc.get("supportedTypes").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    });

    let shim_src = format!(
        r#"// Wrapped open-source Nuvio provider: {scraper_id}
const __module = {{ exports: {{}} }};
(function(module, exports, global, window, require, __dirname, __filename) {{
{raw_js}
}})(__module, __module.exports, globalThis, globalThis, globalThis.require, "/", "/{scraper_id}.js");

const __exported = (__module.exports && typeof __module.exports === "object") ? __module.exports : {{}};
export default {{
    getCatalog: __exported.getCatalog || globalThis.getCatalog,
    getMeta: __exported.getMeta || globalThis.getMeta,
    getStreams: __exported.getStreams || globalThis.getStreams,
    getSubtitles: __exported.getSubtitles || globalThis.getSubtitles,
}};
"#
    );

    let index_src = build_nuvio_adapter_js(&meta_json.to_string());

    let mut files: BTreeMap<String, String> = BTreeMap::new();
    files.insert("provider_shim.js".into(), shim_src);
    files.insert("index.js".into(), index_src);

    let mut hash_buf = serde_json::to_vec(&manifest).unwrap_or_default();
    for (k, v) in &files {
        hash_buf.extend_from_slice(k.as_bytes());
        hash_buf.extend_from_slice(v.as_bytes());
    }
    let sha256 = sha256_hex(&hash_buf);

    Ok(LoadedPackage {
        manifest,
        bundle: PluginBundle::new(files)?,
        icon_png: None,
        sha256,
    })
}

/// Converts a public Stremio `manifest.json` addon into a transparent Vessel REST adapter package.
pub fn build_stremio_addon_package(
    manifest_val: &Value,
    manifest_url: &str,
) -> VesselResult<LoadedPackage> {
    let raw_id = manifest_val
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| VesselError::InvalidInput("Stremio manifest missing 'id'".into()))?;
    let plugin_id = sanitize_plugin_id(raw_id)?;
    let name = manifest_val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(raw_id)
        .to_owned();
    let version = manifest_val
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("1.0.0")
        .to_owned();

    let base_url = manifest_url
        .rsplit_once('/')
        .map(|(dir, _)| dir.trim_end_matches('/').to_owned())
        .unwrap_or_else(|| manifest_url.trim_end_matches('/').to_owned());

    let content_types = map_content_types(manifest_val.get("types").and_then(Value::as_array));
    let capabilities = map_capabilities(
        manifest_val.get("resources").and_then(Value::as_array),
        &content_types,
    );
    let domains = extract_literal_https_domains("", &[manifest_url]);

    let manifest = PluginManifest {
        schema_version: 1,
        id: plugin_id,
        name,
        version,
        author: None,
        description: manifest_val
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        entry: "index.js".into(),
        icon: manifest_val
            .get("logo")
            .and_then(Value::as_str)
            .map(str::to_owned),
        min_core_version: None,
        capabilities,
        content_types,
        languages: vec!["en".into()],
        domains,
        allow_any_https: false,
        is_adult: manifest_val
            .get("behaviorHints")
            .and_then(|h| h.get("adult"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        files: Vec::new(),
        settings_schema: Vec::new(),
    };
    manifest.validate()?;

    let cfg = serde_json::json!({
        "baseUrl": base_url,
        "catalogs": manifest_val.get("catalogs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "types": manifest_val.get("types").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    });

    let index_src = build_stremio_adapter_js(&cfg.to_string());
    let mut hash_buf = serde_json::to_vec(&manifest).unwrap_or_default();
    hash_buf.extend_from_slice(index_src.as_bytes());
    let sha256 = sha256_hex(&hash_buf);

    let mut files: BTreeMap<String, String> = BTreeMap::new();
    files.insert("index.js".into(), index_src);

    Ok(LoadedPackage {
        manifest,
        bundle: PluginBundle::new(files)?,
        icon_png: None,
        sha256,
    })
}

fn build_nuvio_adapter_js(meta_json: &str) -> String {
    format!(
        r##"import {{ definePlugin }} from "vessel";
import provider from "./provider_shim.js";

const META = {meta_json};

function mapMediaType(rawType, rawId) {{
    const id = String(rawId || "").toLowerCase();
    if (id.startsWith("tv:") || id.startsWith("tv_") || id.startsWith("iptv_") || id.startsWith("vns::tv::")) {{
        return "live";
    }}
    const t = String(rawType || "").toLowerCase();
    if (t === "movie") return "movie";
    if (t === "channel" || t === "live" || t === "livetv") return "live";
    return "series";
}}

function isAnimeItem(it, fallbackType) {{
    const rawT = String((it && it.type) || fallbackType || "").toLowerCase();
    if (rawT === "anime") return true;
    const types = Array.isArray(META.supportedTypes)
        ? META.supportedTypes.map((t) => String(t).toLowerCase())
        : [];
    const onlyAnime = types.includes("anime") && !types.includes("series") && !types.includes("movie");
    if (onlyAnime) return true;
    if (it && Array.isArray(it.genres) && it.genres.some((g) => String(g).toLowerCase() === "anime")) {{
        return true;
    }}
    return types.includes("anime") && mapMediaType(rawT, it && it.id) !== "movie" && !types.includes("series");
}}

function matchesQueryTokens(title, query) {{
    const q = String(query || "").toLowerCase().trim();
    if (!q) return true;
    const t = String(title || "").toLowerCase();
    if (t.includes(q)) return true;
    const tokens = q.split(/\s+/).filter((tok) => tok.length >= 2);
    return tokens.length > 0 && tokens.every((tok) => t.includes(tok));
}}

function matchesRequestedType(item, requestedType) {{
    if (!requestedType) return true;
    if (requestedType === "movie") return item.type === "movie";
    if (requestedType === "anime") return item.isAnime === true;
    if (requestedType === "series") return item.type === "series" && !item.isAnime;
    return true;
}}

function mapSummary(it, fallbackType) {{
    if (!it || !it.id || !it.name) return null;
    const rawYear = it.year ?? (it.releaseInfo ? parseInt(String(it.releaseInfo), 10) : undefined);
    const parsedRating = it.imdbRating !== undefined ? parseFloat(String(it.imdbRating)) : undefined;
    return {{
        nativeId: String(it.id),
        type: mapMediaType(it.type || fallbackType, it.id),
        isAnime: isAnimeItem(it, fallbackType),
        title: String(it.name),
        poster: it.poster ? String(it.poster) : undefined,
        year: Number.isFinite(rawYear) ? rawYear : undefined,
        rating: Number.isFinite(parsedRating) ? parsedRating : undefined,
        tags: [],
    }};
}}

function encodeStreamRef(mediaType, season, episode, rawId) {{
    return `vns::${{mediaType || "series"}}::${{season || 1}}::${{episode || 1}}::${{rawId}}`;
}}

function decodeStreamRef(streamId) {{
    const s = String(streamId || "");
    if (s.startsWith("vns::")) {{
        const parts = s.split("::");
        if (parts.length >= 5) {{
            return {{
                mediaType: parts[1] || "series",
                season: parseInt(parts[2], 10) || 1,
                episode: parseInt(parts[3], 10) || 1,
                rawId: parts.slice(4).join("::"),
            }};
        }}
    }}
    return {{ mediaType: "series", season: 1, episode: 1, rawId: s }};
}}

function inferStreamKind(s) {{
    const url = String(s.url || "").toLowerCase();
    const fmt = String(s.format || s.type || "").toLowerCase();
    if (s.isHls === true || fmt === "hls" || fmt === "m3u8" || url.includes(".m3u8")) return "hls";
    if (fmt === "mkv" || url.includes(".mkv")) return "mkv";
    return "mp4";
}}

function inferSubtitleFormat(url) {{
    const u = String(url || "").toLowerCase();
    if (u.includes(".ass")) return "ass";
    if (u.includes(".ssa")) return "ssa";
    if (u.includes(".srt")) return "srt";
    return "vtt";
}}

function normalizeCatalogTargets() {{
    const cats = Array.isArray(META.catalogs) ? META.catalogs : [];
    if (cats.length > 0) return cats;
    const types = Array.isArray(META.supportedTypes) && META.supportedTypes.length
        ? META.supportedTypes
        : ["series"];
    return [{{ id: META.id, name: META.id, type: types[0] || "series" }}];
}}

export default definePlugin({{
    async getHomeFeed() {{
        if (typeof provider.getCatalog !== "function") {{
            return {{ sections: [] }};
        }}
        const catalogs = normalizeCatalogTargets().slice(0, 4);
        const sections = [];
        for (const cat of catalogs) {{
            try {{
                const res = await provider.getCatalog({{
                    type: cat.type || "series",
                    id: cat.id || META.id,
                    extra: {{}},
                }});
                const list = Array.isArray(res) ? res : (res && Array.isArray(res.metas) ? res.metas : []);
                const items = list
                    .slice(0, 30)
                    .map((it) => mapSummary(it, cat.type || "series"))
                    .filter(Boolean);
                if (items.length > 0) {{
                    sections.push({{
                        id: String(cat.id || `${{META.id}}-${{sections.length}}`),
                        title: String(cat.name || META.id),
                        layout: "poster",
                        items,
                    }});
                }}
            }} catch (err) {{
                const msg = String(err || "");
                if (msg.includes("DomainBlocked") || msg.includes("AccessDenied")) {{
                    throw err;
                }}
                console.warn(`catalog '${{cat.id || META.id}}' failed: ${{msg}}`);
            }}
        }}
        return {{ sections }};
    }},

    async search(query, filters, _page) {{
        if (typeof provider.getCatalog !== "function") {{
            return {{ items: [], hasMore: false }};
        }}
        const q = String(query || "").trim();
        const requestedType = filters && filters.mediaType
            ? String(filters.mediaType).trim().toLowerCase()
            : "";
        let catalogs = normalizeCatalogTargets();
        if (requestedType === "movie") {{
            const movieCats = catalogs.filter((c) => String(c.type || "").toLowerCase() === "movie");
            if (movieCats.length > 0) catalogs = movieCats;
        }} else if (requestedType === "series" || requestedType === "anime") {{
            const seriesCats = catalogs.filter((c) => String(c.type || "").toLowerCase() !== "movie");
            if (seriesCats.length > 0) catalogs = seriesCats;
        }}
        catalogs = catalogs.slice(0, 3);
        const seen = new Set();
        const items = [];
        for (const cat of catalogs) {{
            try {{
                const res = await provider.getCatalog({{
                    type: cat.type || "series",
                    id: cat.id || META.id,
                    search: q,
                    extra: {{ search: q }},
                }});
                const list = Array.isArray(res) ? res : (res && Array.isArray(res.metas) ? res.metas : []);
                const mappedList = list.map((raw) => mapSummary(raw, cat.type || "series")).filter(Boolean);
                const hits = q ? mappedList.filter((m) => matchesQueryTokens(m.title, q)).length : mappedList.length;
                const mustFilterByQuery = q.length > 0 && mappedList.length >= 6 && (hits === 0 || (hits * 100) / mappedList.length < 40);
                for (const mapped of mappedList) {{
                    if (seen.has(mapped.nativeId)) continue;
                    if (!matchesRequestedType(mapped, requestedType)) continue;
                    if (mustFilterByQuery && !matchesQueryTokens(mapped.title, q)) continue;
                    seen.add(mapped.nativeId);
                    items.push(mapped);
                    if (items.length >= 40) break;
                }}
            }} catch {{
                // Ignore catalog error during search fan-out
            }}
            if (items.length >= 40) break;
        }}
        return {{ items, hasMore: false }};
    }},

    async getDetails(nativeId) {{
        const fallbackType = String(nativeId).includes("movie")
            ? "movie"
            : (String(nativeId).startsWith("tv:") || String(nativeId).startsWith("iptv_"))
                ? "live"
                : "series";

        let meta = null;
        if (typeof provider.getMeta === "function") {{
            try {{
                const res = await provider.getMeta(nativeId);
                meta = res && res.meta !== undefined ? res.meta : res;
                if (!meta) {{
                    const resObj = await provider.getMeta({{ id: nativeId, type: fallbackType }});
                    meta = resObj && resObj.meta !== undefined ? resObj.meta : resObj;
                }}
            }} catch {{
                meta = null;
            }}
        }}

        const mediaType = mapMediaType((meta && meta.type) || fallbackType, (meta && meta.id) || nativeId);
        const rawVideos = meta && Array.isArray(meta.videos) ? meta.videos : [];
        const episodes = rawVideos.map((v, idx) => {{
            const s = parseInt(v.season, 10) || 1;
            const e = parseInt(v.episode, 10) || (idx + 1);
            const rawEpId = String(v.id || nativeId);
            return {{
                streamId: encodeStreamRef(mediaType === "live" ? "tv" : mediaType, s, e, rawEpId),
                season: s,
                episode: e,
                title: v.title || v.name || `Episode ${{e}}`,
                overview: v.overview || undefined,
                thumbnail: v.thumbnail ? String(v.thumbnail) : undefined,
                airDate: v.released ? String(v.released) : undefined,
                tags: [],
            }};
        }});

        if (episodes.length === 0) {{
            episodes.push({{
                streamId: encodeStreamRef(mediaType === "live" ? "tv" : mediaType, 1, 1, nativeId),
                season: 1,
                episode: 1,
                title: (meta && meta.name) ? String(meta.name) : "Watch Stream",
                tags: [],
            }});
        }}

        const rawYear = meta && meta.year ? parseInt(String(meta.year), 10) : undefined;
        return {{
            nativeId: String((meta && meta.id) || nativeId),
            type: mediaType,
            isAnime: isAnimeItem(meta, mediaType),
            title: String((meta && meta.name) || nativeId),
            year: Number.isFinite(rawYear) ? rawYear : undefined,
            synopsis: meta && meta.description ? String(meta.description) : undefined,
            poster: meta && meta.poster ? String(meta.poster) : undefined,
            backdrop: meta && (meta.background || meta.backdrop) ? String(meta.background || meta.backdrop) : undefined,
            genres: meta && Array.isArray(meta.genres) ? meta.genres.map(String) : [],
            externalIds: {{}},
            episodes,
        }};
    }},

    async getStreams(streamId) {{
        if (typeof provider.getStreams !== "function") {{
            return {{ streams: [], subtitles: [] }};
        }}
        const ref = decodeStreamRef(streamId);
        const mediaArg = ref.mediaType === "series" ? "tv" : ref.mediaType;
        const raw = await provider.getStreams(ref.rawId, mediaArg, ref.season, ref.episode);
        const list = Array.isArray(raw) ? raw : (raw && Array.isArray(raw.streams) ? raw.streams : []);

        const streams = [];
        for (const s of list) {{
            if (!s || !s.url) continue;
            const kind = inferStreamKind(s);
            const labelParts = [s.name || s.server, s.title].filter(Boolean).map(String);
            const label = Array.from(new Set(labelParts)).join(" — ") || String(s.url);
            const subs = Array.isArray(s.subtitles)
                ? s.subtitles.filter((sub) => sub && sub.url).map((sub) => ({{
                    source: {{ kind: "url", url: String(sub.url), headers: {{}} }},
                    lang: String(sub.lang || "und"),
                    label: String(sub.label || sub.name || sub.lang || "Subtitle"),
                    format: inferSubtitleFormat(sub.url),
                }}))
                : [];
            streams.push({{
                url: String(s.url),
                kind,
                quality: s.quality ? String(s.quality) : undefined,
                label,
                flags: [],
                headers: (s.headers && typeof s.headers === "object") ? s.headers : {{}},
                subtitles: subs,
            }});
        }}
        return {{ streams, subtitles: [] }};
    }},
}});
"##
    )
}

fn build_stremio_adapter_js(cfg_json: &str) -> String {
    format!(
        r##"import {{ definePlugin, http }} from "vessel";

const CFG = {cfg_json};

async function fetchJsonOrNull(url) {{
    const resp = await http.get(url);
    if (resp.status < 200 || resp.status >= 300) return null;
    try {{ return resp.json(); }} catch {{ return null; }}
}}

function mapType(rawType) {{
    const t = String(rawType || "").toLowerCase();
    if (t === "movie") return "movie";
    if (t === "channel" || t === "tv") return "live";
    return "series";
}}

function mapSummary(it, fallbackType) {{
    if (!it || !it.id || !it.name) return null;
    const year = it.releaseInfo ? parseInt(String(it.releaseInfo), 10) : undefined;
    const rawType = String(it.type || fallbackType || "").toLowerCase();
    const isAnime = rawType === "anime" || (Array.isArray(it.genres) && it.genres.some((g) => String(g).toLowerCase() === "anime"));
    return {{
        nativeId: `${{fallbackType}}::${{it.id}}`,
        type: mapType(it.type || fallbackType),
        isAnime,
        title: String(it.name),
        poster: it.poster ? String(it.poster) : undefined,
        year: Number.isFinite(year) ? year : undefined,
        tags: [],
    }};
}}

function matchesStremioQuery(title, query) {{
    const q = String(query || "").toLowerCase().trim();
    if (!q) return true;
    const t = String(title || "").toLowerCase();
    if (t.includes(q)) return true;
    const tokens = q.split(/\s+/).filter((tok) => tok.length >= 2);
    return tokens.length > 0 && tokens.every((tok) => t.includes(tok));
}}

function splitTypedId(compositeId) {{
    const s = String(compositeId || "");
    const idx = s.indexOf("::");
    if (idx > 0) {{
        return {{ stype: s.slice(0, idx), rawId: s.slice(idx + 2) }};
    }}
    return {{ stype: (CFG.types && CFG.types[0]) || "movie", rawId: s }};
}}

export default definePlugin({{
    async getHomeFeed() {{
        const catalogs = Array.isArray(CFG.catalogs) ? CFG.catalogs.slice(0, 5) : [];
        const sections = [];
        for (const cat of catalogs) {{
            const url = `${{CFG.baseUrl}}/catalog/${{encodeURIComponent(cat.type)}}/${{encodeURIComponent(cat.id)}}.json`;
            const data = await fetchJsonOrNull(url);
            const metas = data && Array.isArray(data.metas) ? data.metas : [];
            const items = metas.slice(0, 30).map((m) => mapSummary(m, cat.type)).filter(Boolean);
            if (items.length > 0) {{
                sections.push({{
                    id: String(cat.id),
                    title: String(cat.name || cat.id),
                    layout: "poster",
                    items,
                }});
            }}
        }}
        return {{ sections }};
    }},

    async search(query, filters, _page) {{
        const q = String(query || "").trim();
        const requestedType = filters && filters.mediaType
            ? String(filters.mediaType).trim().toLowerCase()
            : "";
        let catalogs = Array.isArray(CFG.catalogs) ? CFG.catalogs : [];
        if (requestedType === "movie") {{
            const movieCats = catalogs.filter((c) => String(c.type || "").toLowerCase() === "movie");
            if (movieCats.length > 0) catalogs = movieCats;
        }} else if (requestedType === "series" || requestedType === "anime") {{
            const seriesCats = catalogs.filter((c) => String(c.type || "").toLowerCase() !== "movie");
            if (seriesCats.length > 0) catalogs = seriesCats;
        }}
        catalogs = catalogs.slice(0, 4);
        const seen = new Set();
        const items = [];
        for (const cat of catalogs) {{
            let data = await fetchJsonOrNull(
                `${{CFG.baseUrl}}/catalog/${{encodeURIComponent(cat.type)}}/${{encodeURIComponent(cat.id)}}/search=${{encodeURIComponent(q)}}.json`,
            );
            let clientFilter = false;
            if (!data) {{
                data = await fetchJsonOrNull(
                    `${{CFG.baseUrl}}/catalog/${{encodeURIComponent(cat.type)}}/${{encodeURIComponent(cat.id)}}.json`,
                );
                clientFilter = true;
            }}
            const metas = data && Array.isArray(data.metas) ? data.metas : [];
            const mappedList = metas.map((m) => mapSummary(m, cat.type)).filter(Boolean);
            const hits = q ? mappedList.filter((m) => matchesStremioQuery(m.title, q)).length : mappedList.length;
            const mustFilter = clientFilter || (q.length > 0 && mappedList.length >= 6 && (hits === 0 || (hits * 100) / mappedList.length < 40));
            for (const mapped of mappedList) {{
                if (seen.has(mapped.nativeId)) continue;
                if (requestedType === "movie" && mapped.type !== "movie") continue;
                if (requestedType === "anime" && !mapped.isAnime) continue;
                if (requestedType === "series" && (mapped.type !== "series" || mapped.isAnime)) continue;
                if (mustFilter && q && !matchesStremioQuery(mapped.title, q)) continue;
                seen.add(mapped.nativeId);
                items.push(mapped);
                if (items.length >= 40) break;
            }}
            if (items.length >= 40) break;
        }}
        return {{ items, hasMore: false }};
    }},

    async getDetails(compositeId) {{
        const {{ stype, rawId }} = splitTypedId(compositeId);
        let data = await fetchJsonOrNull(`${{CFG.baseUrl}}/meta/${{encodeURIComponent(stype)}}/${{encodeURIComponent(rawId)}}.json`);
        if (!data) {{
            data = await fetchJsonOrNull(`${{CFG.baseUrl}}/meta/${{stype}}/${{rawId}}.json`);
        }}
        const meta = (data && data.meta) || {{ id: rawId, name: rawId, type: stype }};
        const rawVideos = Array.isArray(meta.videos) ? meta.videos : [];
        const episodes = rawVideos.map((v, idx) => ({{
            streamId: `${{stype}}::${{v.id || rawId}}`,
            season: parseInt(v.season, 10) || 1,
            episode: parseInt(v.episode, 10) || (idx + 1),
            title: String(v.title || v.name || `Episode ${{idx + 1}}`),
            overview: v.overview ? String(v.overview) : undefined,
            thumbnail: v.thumbnail ? String(v.thumbnail) : undefined,
            tags: [],
        }}));
        if (episodes.length === 0) {{
            episodes.push({{
                streamId: `${{stype}}::${{rawId}}`,
                season: 1,
                episode: 1,
                title: String(meta.name || "Watch Stream"),
                tags: [],
            }});
        }}
        return {{
            nativeId: compositeId,
            type: mapType(meta.type || stype),
            isAnime: false,
            title: String(meta.name || rawId),
            synopsis: meta.description ? String(meta.description) : undefined,
            poster: meta.poster ? String(meta.poster) : undefined,
            backdrop: meta.background ? String(meta.background) : undefined,
            genres: Array.isArray(meta.genres) ? meta.genres.map(String) : [],
            externalIds: {{}},
            episodes,
        }};
    }},

    async getStreams(streamId) {{
        const {{ stype, rawId }} = splitTypedId(streamId);
        let data = await fetchJsonOrNull(`${{CFG.baseUrl}}/stream/${{encodeURIComponent(stype)}}/${{encodeURIComponent(rawId)}}.json`);
        if (!data) {{
            data = await fetchJsonOrNull(`${{CFG.baseUrl}}/stream/${{stype}}/${{rawId}}.json`);
        }}
        const list = data && Array.isArray(data.streams) ? data.streams : [];
        const streams = list
            .filter((s) => s && s.url)
            .map((s) => ({{
                url: String(s.url),
                kind: String(s.url).toLowerCase().includes(".m3u8") ? "hls" : "mp4",
                label: String(s.name || s.title || s.url),
                flags: [],
                headers: (s.behaviorHints && s.behaviorHints.proxyHeaders && s.behaviorHints.proxyHeaders.request) || {{}},
                subtitles: [],
            }}));
        return {{ streams, subtitles: [] }};
    }},
}});
"##
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::Router;
    use axum::response::Html;
    use axum::routing::get;
    use tokio::net::TcpListener;
    use vessel_domain::{ContentProvider, PluginId};

    use super::{
        ExternalManifestKind, build_nuvio_scraper_package, check_cloudstream_closed_plugins,
        parse_external_manifest, verify_open_source_js,
    };
    use crate::{
        DomainPolicy, HostServices, QuickJsContentProvider, SandboxLimits, TsCompileCache,
    };

    struct NoopStorage;

    #[async_trait::async_trait]
    impl vessel_domain::StoragePort for NoopStorage {
        async fn get_setting(&self, _: &str) -> vessel_domain::VesselResult<Option<String>> {
            Ok(None)
        }
        async fn set_setting(&self, _: &str, _: &str) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn get_plugin_kv(
            &self,
            _: &PluginId,
            _: &str,
        ) -> vessel_domain::VesselResult<Option<String>> {
            Ok(None)
        }
        async fn set_plugin_kv(
            &self,
            _: &PluginId,
            _: &str,
            _: &str,
        ) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn remove_plugin_kv(&self, _: &PluginId, _: &str) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn is_domain_allowed(
            &self,
            _: &PluginId,
            _: &str,
        ) -> vessel_domain::VesselResult<bool> {
            Ok(true)
        }
        async fn grant_domain(
            &self,
            _: &PluginId,
            _: &str,
            _: vessel_domain::DomainGrantSource,
        ) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn get_progress(
            &self,
            _: &vessel_domain::MediaRef,
            _: u32,
            _: u32,
        ) -> vessel_domain::VesselResult<Option<vessel_domain::PlaybackProgress>> {
            Ok(None)
        }
        async fn upsert_progress(
            &self,
            _: &vessel_domain::PlaybackProgress,
        ) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn list_library(
            &self,
            _: Option<vessel_domain::LibraryStatus>,
        ) -> vessel_domain::VesselResult<Vec<vessel_domain::LibraryItem>> {
            Ok(Vec::new())
        }
        async fn upsert_library_item(
            &self,
            _: &vessel_domain::LibraryItem,
        ) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
        async fn remove_library_item(
            &self,
            _: &vessel_domain::MediaRef,
        ) -> vessel_domain::VesselResult<()> {
            Ok(())
        }
    }

    #[test]
    fn blocks_obfuscated_js_and_cloudstream_cs3_repos() {
        let obfuscated = "_0x1a2b _0x1a2c _0x1a2d _0x1a2e _0x1a2f _0x1b2a _0x1b2b _0x1b2c _0x1b2d _0x1b2e _0x1b2f _0x1c2a _0x1c2b _0x1c2c _0x1c2d _0x1c2e";
        assert!(verify_open_source_js(obfuscated, "https://evil.example/a.js").is_err());

        let cs3_list = br#"[{"name":"ClosedPlugin","url":"https://example.com/Plugin.cs3"}]"#;
        let err = check_cloudstream_closed_plugins(cs3_list, "https://example.com/plugins.json")
            .unwrap_err();
        assert!(err.to_string().contains(".cs3"));
    }

    #[tokio::test]
    async fn parses_and_executes_nuvio_scraper_with_cheerio_and_fetch() {
        let app = Router::new().route(
            "/catalog",
            get(|| async {
                Html(
                    r#"<html><body><div class="card" data-id="puhutv:show:test"><a href="/show">Test Series</a><img src="https://img.example/p.jpg"/></div></body></html>"#,
                )
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let manifest_json = serde_json::json!({
            "name": "Anthology",
            "scrapers": [{
                "id": "demo_nuvio",
                "name": "Demo Nuvio",
                "version": "1.0.0",
                "filename": "providers/demo_nuvio.js",
                "supportedTypes": ["series"],
                "contentLanguage": ["tr"],
                "catalogs": [{ "type": "series", "id": "main", "name": "Featured Shows" }]
            }]
        });

        let ext = parse_external_manifest(
            manifest_json.to_string().as_bytes(),
            "https://raw.githubusercontent.com/falsisdev/anthology/main/manifest.json",
        )
        .expect("parse nuvio")
        .expect("nuvio kind");
        match ext {
            ExternalManifestKind::NuvioRepo(repo) => {
                assert_eq!(repo.plugins.len(), 1);
                assert_eq!(repo.plugins[0].id.as_str(), "demo_nuvio");
            }
            _ => panic!("expected NuvioRepo"),
        }

        let scraper_js = format!(
            r#"
            const cheerio = require("cheerio-without-node-native");
            async function getCatalog(args) {{
                const res = await fetch("http://{addr}/catalog");
                const html = await res.text();
                const $ = cheerio.load(html);
                const metas = [];
                $(".card").each(function(i, el) {{
                    metas.push({{
                        id: $(el).attr("data-id"),
                        type: "series",
                        name: $(el).find("a").text().trim(),
                        poster: $(el).find("img").attr("src")
                    }});
                }});
                return {{ metas }};
            }}
            async function getMeta(id) {{
                return {{ meta: {{ id, type: "series", name: "Test Series", videos: [{{ id: id + ":1:1", season: 1, episode: 1, title: "Pilot" }}] }} }};
            }}
            async function getStreams(id) {{
                return [{{ url: "https://cdn.example/video.mp4", name: "CDN", title: "1080p", quality: "1080p", format: "mp4" }}];
            }}
            module.exports = {{ getCatalog, getMeta, getStreams }};
            "#
        );

        let pkg = build_nuvio_scraper_package(
            &manifest_json,
            "https://raw.githubusercontent.com/falsisdev/anthology/main/manifest.json",
            "demo_nuvio",
            &scraper_js,
            "https://raw.githubusercontent.com/falsisdev/anthology/main/providers/demo_nuvio.js",
        )
        .expect("build pkg");

        let policy = DomainPolicy::new(pkg.manifest.id.clone(), vec!["127.0.0.1".into()], false)
            .with_loopback_for_tests();
        let host = Arc::new(
            HostServices::new(
                pkg.manifest.id.clone(),
                Arc::new(policy),
                Arc::new(NoopStorage),
                None,
                Arc::new(TsCompileCache::default()),
            )
            .expect("host"),
        );

        let provider = QuickJsContentProvider::load(
            pkg.manifest,
            Arc::new(pkg.bundle),
            host,
            SandboxLimits::default(),
        )
        .await
        .expect("load nuvio");

        let feed = provider.home_feed(None).await.expect("home feed");
        assert_eq!(feed.sections.len(), 1);
        assert_eq!(feed.sections[0].items[0].title, "Test Series");

        let detail = provider.details("puhutv:show:test").await.expect("details");
        assert_eq!(detail.episodes.len(), 1);

        let streams = provider
            .streams(&detail.episodes[0].stream_id)
            .await
            .expect("streams");
        assert_eq!(streams.streams.len(), 1);
        assert_eq!(streams.streams[0].url, "https://cdn.example/video.mp4");
    }

    #[tokio::test]
    #[ignore = "Live network integration probe against gh:falsisdev/anthology"]
    async fn live_probe_anthology() {
        use crate::{ResolvedSource, SourceResolver};
        use std::sync::Arc;
        use vessel_domain::{EngineEvent, EventSink};

        struct PrintSink;
        impl EventSink for PrintSink {
            fn emit(&self, ev: EngineEvent) {
                if let EngineEvent::PluginLog {
                    plugin_id,
                    level,
                    message,
                } = ev
                {
                    println!("  [{plugin_id}:{level:?}] {message}");
                }
            }
        }
        let sink: Arc<dyn EventSink> = Arc::new(PrintSink);

        let resolver = SourceResolver::new();
        let resolved = resolver
            .resolve("gh:falsisdev/anthology")
            .await
            .expect("resolve anthology");
        let repo = match resolved {
            ResolvedSource::Repository { repo, .. } => repo,
            _ => panic!("expected Repository"),
        };
        println!("REPO: {} ({} plugins)", repo.name, repo.plugins.len());

        // Also test Stremio folder `gh:falsisdev/anthology/stremio`
        let stremio_res = resolver
            .resolve("gh:falsisdev/anthology/stremio")
            .await
            .expect("resolve anthology/stremio");
        if let ResolvedSource::Package { package, .. } = stremio_res {
            let policy = DomainPolicy::new(
                package.manifest.id.clone(),
                package.manifest.domains.clone(),
                false,
            );
            let host = Arc::new(
                HostServices::new(
                    package.manifest.id.clone(),
                    Arc::new(policy),
                    Arc::new(NoopStorage),
                    None,
                    Arc::new(TsCompileCache::default()),
                )
                .unwrap(),
            );
            let prov = QuickJsContentProvider::load(
                package.manifest.clone(),
                Arc::new(package.bundle),
                host,
                SandboxLimits::default(),
            )
            .await
            .expect("load stremio");
            let feed = prov.home_feed(None).await.expect("stremio feed");
            let first_item = &feed.sections[0].items[0];
            let det = prov
                .details(&first_item.native_id)
                .await
                .expect("stremio det");
            let strm = prov
                .streams(&det.episodes[0].stream_id)
                .await
                .expect("stremio strm");
            println!(
                "STREMIO OK: sections={} first='{}' streams={}",
                feed.sections.len(),
                first_item.title,
                strm.streams.len()
            );
            prov.shutdown().await;
        }

        // Probe every enabled scraper in anthology manifest
        for entry in &repo.plugins {
            if entry
                .description
                .as_deref()
                .unwrap_or("")
                .starts_with("[Disabled")
            {
                continue;
            }
            let Some(ref src) = entry.source_path else {
                continue;
            };
            match resolver.resolve(src).await {
                Ok(ResolvedSource::Package { package, .. }) => {
                    let policy = DomainPolicy::new(
                        package.manifest.id.clone(),
                        package.manifest.domains.clone(),
                        true,
                    );
                    let host = Arc::new(
                        HostServices::new(
                            package.manifest.id.clone(),
                            Arc::new(policy),
                            Arc::new(NoopStorage),
                            Some(Arc::clone(&sink)),
                            Arc::new(TsCompileCache::default()),
                        )
                        .unwrap(),
                    );
                    match QuickJsContentProvider::load(
                        package.manifest.clone(),
                        Arc::new(package.bundle),
                        host,
                        SandboxLimits::default(),
                    )
                    .await
                    {
                        Ok(prov) => {
                            let feed_res = prov.home_feed(None).await;
                            match feed_res {
                                Ok(feed) => {
                                    let count: usize =
                                        feed.sections.iter().map(|s| s.items.len()).sum();
                                    let sample_id = feed
                                        .sections
                                        .first()
                                        .and_then(|s| s.items.first())
                                        .map(|i| i.native_id.clone());
                                    if let Some(nid) = sample_id {
                                        let det_res = prov.details(&nid).await;
                                        match det_res {
                                            Ok(det) => {
                                                if let Some(ep) = det.episodes.first() {
                                                    let strm = prov.streams(&ep.stream_id).await;
                                                    match strm {
                                                        Ok(sr) => println!(
                                                            "OK   {:24} | feed_items={:3} | eps={:2} | streams={}",
                                                            entry.id.as_str(),
                                                            count,
                                                            det.episodes.len(),
                                                            sr.streams.len()
                                                        ),
                                                        Err(e) => println!(
                                                            "ERR  {:24} | feed_items={:3} | eps={:2} | stream_err={}",
                                                            entry.id.as_str(),
                                                            count,
                                                            det.episodes.len(),
                                                            e
                                                        ),
                                                    }
                                                } else {
                                                    println!(
                                                        "WARN {:24} | feed_items={:3} | eps=0",
                                                        entry.id.as_str(),
                                                        count
                                                    );
                                                }
                                            }
                                            Err(e) => println!(
                                                "ERR  {:24} | feed_items={:3} | detail_err={}",
                                                entry.id.as_str(),
                                                count,
                                                e
                                            ),
                                        }
                                    } else {
                                        println!(
                                            "EMPTY {:23} | sections={}",
                                            entry.id.as_str(),
                                            feed.sections.len()
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("ERR  {:24} | feed_err={}", entry.id.as_str(), e)
                                }
                            }
                            prov.shutdown().await;
                        }
                        Err(e) => println!("LOAD_ERR {:20} | {}", entry.id.as_str(), e),
                    }
                }
                Ok(_) => {}
                Err(e) => println!("RES_ERR {:21} | {}", entry.id.as_str(), e),
            }
        }
    }
}
