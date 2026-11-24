//! Source Resolver (`https://`, `gh:`, `gl:`, `registry.json` + HTTP `301/302` shortcodes, `vessel://install`, Dev Mode).

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::header::LOCATION;
use reqwest::redirect::Policy;
use url::{Host, Url};
use vessel_domain::{
    PluginManifest, RegistryIndex, RepoIndex, RepoPluginEntry, VesselError, VesselResult,
};

use crate::compat::{
    ExternalManifestKind, NUVIO_SCHEME_PREFIX, build_nuvio_scraper_package,
    check_cloudstream_closed_plugins, parse_external_manifest, verify_open_source_js,
};
use crate::module_loader::PluginBundle;
use crate::package::{
    LoadedPackage, collect_relative_imports, load_plugin_dir, load_plugin_package, verify_sha256,
};
use crate::security::is_forbidden_ip;
use crate::ts_strip::sha256_hex;

const RESOLVER_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 8;
const MAX_PLUGIN_LIST_DEPTH: usize = 1;
const MAX_SHORTCODE_HOPS: usize = 3;
const DEFAULT_BRANCH_CANDIDATES: &[&str] = &["main", "master"];
const TRY_MODULE_EXTENSIONS: &[&str] =
    &["", ".ts", ".mts", ".js", ".mjs", "/index.ts", "/index.js"];

/// Official Vessel shortcode registry hosted in the main repository.
pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/falsisdev/vessel/main/registry.json";

/// Built-in official shortcodes kept as a zero-network fallback for `registry.json`.
pub fn builtin_shortcode(code: &str) -> Option<&'static str> {
    match code.trim().to_ascii_lowercase().as_str() {
        "anthology" | "tr" => Some("gh:falsisdev/anthology"),
        _ => None,
    }
}

/// Core-controlled trust badge (never read from `manifest.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTrustBadge {
    Official,
    Community,
}

/// Normalized representation of any user-entered repository or plugin source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSource {
    DirectUrl(String),
    GitHub {
        owner: String,
        repo: String,
        git_ref: Option<String>,
        subpath: String,
    },
    GitLab {
        project_path: String,
        git_ref: Option<String>,
        subpath: String,
    },
    NuvioScraper {
        manifest_url: String,
        scraper_id: String,
    },
    Shortcode(String),
    LocalPath(PathBuf),
}

/// Result of resolving and fetching a source target.
#[derive(Debug, Clone)]
pub enum ResolvedSource {
    Repository {
        resolved_url: String,
        base_raw_url: Option<String>,
        repo: RepoIndex,
        badge: SourceTrustBadge,
    },
    Package {
        resolved_url: String,
        package: LoadedPackage,
        badge: SourceTrustBadge,
    },
}

fn strip_vessel_install_uri(raw: &str) -> VesselResult<Option<String>> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("vessel://") {
        return Ok(None);
    }
    let parsed = Url::parse(trimmed).map_err(|err| {
        VesselError::InvalidInput(format!("invalid vessel:// deep link '{trimmed}': {err}"))
    })?;
    if parsed.host_str() != Some("install") {
        return Err(VesselError::InvalidInput(format!(
            "unsupported vessel:// action '{}'; expected vessel://install?src=...",
            parsed.host_str().unwrap_or_default()
        )));
    }
    let src = parsed
        .query_pairs()
        .find(|(k, _)| k == "src")
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            VesselError::InvalidInput("vessel://install missing 'src' query param".into())
        })?;
    Ok(Some(src))
}

fn split_repo_ref_and_subpath(after_owner: &str) -> (&str, Option<String>, String) {
    let trimmed = after_owner.trim_matches('/');
    let (head, tail) = match trimmed.split_once('/') {
        Some((repo_part, rest)) => (repo_part, rest.trim_matches('/').to_owned()),
        None => (trimmed, String::new()),
    };

    if let Some((repo, ref_name)) = head.split_once('@') {
        return (repo, Some(ref_name.to_owned()), tail);
    }
    (head, None, tail)
}

/// Parses a direct URL, `gh:`/`gl:` shorthand, `vessel://install?src=...` URI, local path, or shortcode.
pub fn parse_source_input(raw: &str) -> VesselResult<ParsedSource> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(VesselError::InvalidInput(
            "source input cannot be empty".into(),
        ));
    }

    if let Some(unwrapped) = strip_vessel_install_uri(trimmed)? {
        if unwrapped.starts_with("vessel://") {
            return Err(VesselError::InvalidInput(
                "nested vessel:// deep links are not allowed".into(),
            ));
        }
        return parse_source_input(&unwrapped);
    }

    if let Some(nuvio_rest) = trimmed.strip_prefix(NUVIO_SCHEME_PREFIX) {
        let Some((manifest_url, scraper_id)) = nuvio_rest.rsplit_once('#') else {
            return Err(VesselError::InvalidInput(format!(
                "invalid internal Nuvio scraper reference '{trimmed}'"
            )));
        };
        if manifest_url.trim().is_empty() || scraper_id.trim().is_empty() {
            return Err(VesselError::InvalidInput(format!(
                "invalid internal Nuvio scraper reference '{trimmed}'"
            )));
        }
        return Ok(ParsedSource::NuvioScraper {
            manifest_url: manifest_url.to_owned(),
            scraper_id: scraper_id.to_owned(),
        });
    }

    if let Some(stremio_rest) = trimmed.strip_prefix("stremio://") {
        let https_url = format!("https://{stremio_rest}");
        let parsed = Url::parse(&https_url).map_err(|err| {
            VesselError::InvalidInput(format!("invalid stremio:// URL '{trimmed}': {err}"))
        })?;
        return Ok(ParsedSource::DirectUrl(parsed.to_string()));
    }

    if let Some(gh_rest) = trimmed.strip_prefix("gh:") {
        let clean = gh_rest.trim_matches('/');
        let Some((owner, after_owner)) = clean.split_once('/') else {
            return Err(VesselError::InvalidInput(format!(
                "invalid GitHub shorthand '{trimmed}'; expected gh:owner/repo[/subpath]"
            )));
        };
        let (repo, git_ref, subpath) = split_repo_ref_and_subpath(after_owner);
        if owner.is_empty() || repo.is_empty() {
            return Err(VesselError::InvalidInput(format!(
                "invalid GitHub shorthand '{trimmed}'"
            )));
        }
        return Ok(ParsedSource::GitHub {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            git_ref,
            subpath,
        });
    }

    if let Some(gl_rest) = trimmed.strip_prefix("gl:") {
        let clean = gl_rest.trim_matches('/');
        let (project_part, git_ref, subpath) = if let Some((proj, after_at)) = clean.split_once('@')
        {
            if let Some((branch, sub)) = after_at.split_once('/') {
                (
                    proj.trim_matches('/'),
                    Some(branch.to_owned()),
                    sub.trim_matches('/').to_owned(),
                )
            } else {
                (
                    proj.trim_matches('/'),
                    Some(after_at.to_owned()),
                    String::new(),
                )
            }
        } else {
            let mut segs: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
            if segs.len() < 2 {
                return Err(VesselError::InvalidInput(format!(
                    "invalid GitLab shorthand '{trimmed}'; expected gl:group/project[/subpath]"
                )));
            }
            let subpath = if segs.len() > 2 {
                segs.split_off(2).join("/")
            } else {
                String::new()
            };
            let joined = segs.join("/");
            return Ok(ParsedSource::GitLab {
                project_path: joined,
                git_ref: None,
                subpath,
            });
        };

        if !project_part.contains('/') {
            return Err(VesselError::InvalidInput(format!(
                "invalid GitLab shorthand '{trimmed}'"
            )));
        }
        return Ok(ParsedSource::GitLab {
            project_path: project_part.to_owned(),
            git_ref,
            subpath,
        });
    }

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        let parsed = Url::parse(trimmed).map_err(|err| {
            VesselError::InvalidInput(format!("invalid source URL '{trimmed}': {err}"))
        })?;
        // Convert plain `https://github.com/owner/repo[/tree/branch/subpath]` repository URLs
        if parsed
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("github.com"))
        {
            let segs: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.filter(|p| !p.is_empty()).collect())
                .unwrap_or_default();
            if segs.len() == 2 {
                return Ok(ParsedSource::GitHub {
                    owner: segs[0].to_owned(),
                    repo: segs[1].trim_end_matches(".git").to_owned(),
                    git_ref: None,
                    subpath: String::new(),
                });
            }
            if segs.len() >= 4 && segs[2] == "tree" {
                return Ok(ParsedSource::GitHub {
                    owner: segs[0].to_owned(),
                    repo: segs[1].trim_end_matches(".git").to_owned(),
                    git_ref: Some(segs[3].to_owned()),
                    subpath: segs[4..].join("/"),
                });
            }
        }
        return Ok(ParsedSource::DirectUrl(parsed.to_string()));
    }

    if trimmed.starts_with('/')
        || trimmed.starts_with("./")
        || trimmed.starts_with("../")
        || trimmed.contains('\\')
        || Path::new(trimmed).exists()
    {
        return Ok(ParsedSource::LocalPath(PathBuf::from(trimmed)));
    }

    let valid_shortcode = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !valid_shortcode {
        return Err(VesselError::InvalidInput(format!(
            "unrecognized source format '{trimmed}'"
        )));
    }

    Ok(ParsedSource::Shortcode(trimmed.to_ascii_lowercase()))
}

/// Core-controlled trust badge derivation (only official `gh:falsisdev/...` sources receive `Official`).
pub fn badge_for_source(source: &ParsedSource) -> SourceTrustBadge {
    match source {
        ParsedSource::GitHub { owner, .. } if owner.eq_ignore_ascii_case("falsisdev") => {
            SourceTrustBadge::Official
        }
        ParsedSource::NuvioScraper { manifest_url, .. } if manifest_url.contains("/falsisdev/") => {
            SourceTrustBadge::Official
        }
        ParsedSource::DirectUrl(url)
            if url.starts_with("https://raw.githubusercontent.com/falsisdev/")
                || url.starts_with("https://falsisdev.github.io/") =>
        {
            SourceTrustBadge::Official
        }
        _ => SourceTrustBadge::Community,
    }
}

fn dir_base_url(url: &str) -> Option<String> {
    let (prefix, _) = url.rsplit_once('/')?;
    Some(prefix.to_owned())
}

fn join_base_path(base: &str, rel: &str) -> String {
    let base_trim = base.trim_end_matches('/');
    let rel_trim = rel.trim_start_matches("./").trim_start_matches('/');
    if rel_trim.is_empty() {
        base_trim.to_owned()
    } else {
        format!("{base_trim}/{rel_trim}")
    }
}

/// Multi-format source resolver (`https://`, `gh:`, `gl:`, `registry.json` + HTTP shortcodes, Dev Mode).
#[derive(Debug, Clone)]
pub struct SourceResolver {
    client: reqwest::Client,
    github_raw_base: String,
    gitlab_base: String,
    registry_urls: Vec<String>,
    shortener_base_url: Option<String>,
    allow_loopback_for_tests: bool,
}

impl Default for SourceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceResolver {
    /// Creates a production `SourceResolver` backed by `raw.githubusercontent.com` and `gitlab.com`.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(RESOLVER_TIMEOUT)
            .build()
            .expect("build source resolver reqwest client");
        Self {
            client,
            github_raw_base: "https://raw.githubusercontent.com".into(),
            gitlab_base: "https://gitlab.com".into(),
            registry_urls: Vec::new(),
            shortener_base_url: None,
            allow_loopback_for_tests: false,
        }
    }

    /// Configures static `registry.json` endpoints consulted for one-word shortcodes.
    pub fn with_registries(mut self, urls: impl IntoIterator<Item = String>) -> Self {
        self.registry_urls = urls.into_iter().collect();
        self
    }

    /// Enables Vessel's official default `registry.json` endpoint for shortcode resolution.
    pub fn with_default_registries(mut self) -> Self {
        if !self.registry_urls.iter().any(|u| u == DEFAULT_REGISTRY_URL) {
            self.registry_urls.push(DEFAULT_REGISTRY_URL.to_owned());
        }
        self
    }

    /// Configures an optional HTTP shortener base (`<base>/<shortcode>` redirecting via `301/302`).
    pub fn with_shortener_base(mut self, base_url: impl Into<String>) -> Self {
        self.shortener_base_url = Some(base_url.into().trim_end_matches('/').to_owned());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_test_endpoints(
        mut self,
        github_raw_base: impl Into<String>,
        gitlab_base: impl Into<String>,
    ) -> Self {
        self.github_raw_base = github_raw_base.into().trim_end_matches('/').to_owned();
        self.gitlab_base = gitlab_base.into().trim_end_matches('/').to_owned();
        self.allow_loopback_for_tests = true;
        self
    }

    fn validate_url(&self, raw: &str) -> VesselResult<Url> {
        let parsed = Url::parse(raw)
            .map_err(|err| VesselError::InvalidInput(format!("invalid URL '{raw}': {err}")))?;
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return Err(VesselError::InvalidInput(format!(
                "unsupported source URL scheme '{}'",
                parsed.scheme()
            )));
        }
        let host = parsed
            .host()
            .ok_or_else(|| VesselError::InvalidInput(format!("URL missing host: '{raw}'")))?;
        let is_test_loopback = self.allow_loopback_for_tests
            && match host {
                Host::Ipv4(v4) => v4.is_loopback(),
                Host::Ipv6(v6) => v6.is_loopback(),
                Host::Domain(d) => d.eq_ignore_ascii_case("localhost"),
            };

        if !is_test_loopback {
            match host {
                Host::Ipv4(v4) if is_forbidden_ip(IpAddr::V4(v4)) => {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked private IPv4 source '{raw}'"
                    )));
                }
                Host::Ipv6(v6) if is_forbidden_ip(IpAddr::V6(v6)) => {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked private IPv6 source '{raw}'"
                    )));
                }
                Host::Domain(domain)
                    if domain.eq_ignore_ascii_case("localhost")
                        || domain.ends_with(".local")
                        || domain.ends_with(".internal") =>
                {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked local source hostname '{domain}'"
                    )));
                }
                _ => {}
            }
        }
        Ok(parsed)
    }

    async fn fetch_bytes_checked(&self, url: &str) -> VesselResult<(String, Vec<u8>)> {
        let mut current = self.validate_url(url)?;
        for hop in 0..=MAX_REDIRECTS {
            let resp = self
                .client
                .get(current.clone())
                .send()
                .await
                .map_err(|err| VesselError::Network(format!("fetch {current}: {err}")))?;

            if resp.status().is_redirection() {
                if hop == MAX_REDIRECTS {
                    return Err(VesselError::Network("too many HTTP redirects".into()));
                }
                let Some(loc) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok()) else {
                    return Err(VesselError::Network(
                        "redirect missing Location header".into(),
                    ));
                };
                let next = current.join(loc).map_err(|err| {
                    VesselError::InvalidInput(format!("invalid redirect '{loc}': {err}"))
                })?;
                current = self.validate_url(next.as_str())?;
                continue;
            }

            if matches!(resp.status().as_u16(), 401 | 403) {
                return Err(VesselError::InvalidInput(format!(
                    "Source at '{}' is private or access-restricted (HTTP {}). Vessel only adds publicly accessible open-source plugins.",
                    current,
                    resp.status().as_u16()
                )));
            }
            if !resp.status().is_success() {
                return Err(VesselError::NotFound(format!(
                    "HTTP {} fetching {}",
                    resp.status().as_u16(),
                    current
                )));
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|err| VesselError::Network(format!("read {current}: {err}")))?
                .to_vec();
            return Ok((current.to_string(), bytes));
        }

        unreachable!("redirect loop terminates by MAX_REDIRECTS")
    }

    async fn try_fetch_bytes(&self, url: &str) -> VesselResult<Option<(String, Vec<u8>)>> {
        match self.fetch_bytes_checked(url).await {
            Ok(ok) => Ok(Some(ok)),
            Err(VesselError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Resolves any supported source input (`https://`, `gh:`, `gl:`, shortcode, local dir, `vessel://`).
    pub async fn resolve(&self, raw_input: &str) -> VesselResult<ResolvedSource> {
        self.resolve_inner(raw_input, 0).await
    }

    async fn resolve_inner(&self, raw_input: &str, depth: usize) -> VesselResult<ResolvedSource> {
        let parsed = parse_source_input(raw_input)?;
        let badge = badge_for_source(&parsed);

        match parsed {
            ParsedSource::Shortcode(code) => {
                if depth >= MAX_SHORTCODE_HOPS {
                    return Err(VesselError::InvalidInput(format!(
                        "shortcode resolution exceeded max depth for '{code}'"
                    )));
                }
                let mapped = self.lookup_shortcode(&code).await?;
                Box::pin(self.resolve_inner(&mapped, depth + 1)).await
            }
            ParsedSource::LocalPath(path) => self.resolve_local(&path, badge).await,
            ParsedSource::GitHub {
                owner,
                repo,
                git_ref,
                subpath,
            } => {
                let branches: Vec<&str> = match git_ref.as_deref() {
                    Some(r) => vec![r],
                    None => DEFAULT_BRANCH_CANDIDATES.to_vec(),
                };
                let mut candidates = Vec::with_capacity(branches.len());
                for branch in branches {
                    let base = format!("{}/{owner}/{repo}/{branch}", self.github_raw_base);
                    candidates.push(join_base_path(&base, &subpath));
                }
                self.probe_git_raw_bases(&candidates, badge).await
            }
            ParsedSource::GitLab {
                project_path,
                git_ref,
                subpath,
            } => {
                let branches: Vec<&str> = match git_ref.as_deref() {
                    Some(r) => vec![r],
                    None => DEFAULT_BRANCH_CANDIDATES.to_vec(),
                };
                let mut candidates = Vec::with_capacity(branches.len());
                for branch in branches {
                    let base = format!("{}/{project_path}/-/raw/{branch}", self.gitlab_base);
                    candidates.push(join_base_path(&base, &subpath));
                }
                self.probe_git_raw_bases(&candidates, badge).await
            }
            ParsedSource::DirectUrl(url) => self.resolve_direct_url(&url, badge).await,
            ParsedSource::NuvioScraper {
                manifest_url,
                scraper_id,
            } => {
                self.resolve_nuvio_scraper(&manifest_url, &scraper_id, badge)
                    .await
            }
        }
    }

    async fn resolve_nuvio_scraper(
        &self,
        manifest_url: &str,
        scraper_id: &str,
        badge: SourceTrustBadge,
    ) -> VesselResult<ResolvedSource> {
        let (final_manifest_url, manifest_bytes) = self.fetch_bytes_checked(manifest_url).await?;
        let manifest_val: serde_json::Value = serde_json::from_slice(&manifest_bytes)
            .map_err(|err| VesselError::InvalidInput(format!("invalid Nuvio manifest: {err}")))?;
        let filename = manifest_val
            .get("scrapers")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(scraper_id))
            })
            .and_then(|s| s.get("filename").and_then(|v| v.as_str()))
            .ok_or_else(|| {
                VesselError::NotFound(format!(
                    "scraper '{scraper_id}' missing filename in '{final_manifest_url}'"
                ))
            })?;

        let js_url = if filename.starts_with("https://") || filename.starts_with("http://") {
            filename.to_owned()
        } else {
            let base = dir_base_url(&final_manifest_url).ok_or_else(|| {
                VesselError::InvalidInput("cannot resolve Nuvio scraper relative filename".into())
            })?;
            join_base_path(&base, filename)
        };

        let (final_js_url, js_bytes) = self.fetch_bytes_checked(&js_url).await?;
        let raw_js = String::from_utf8(js_bytes).map_err(|err| {
            VesselError::InvalidInput(format!(
                "plugin source at '{final_js_url}' is not valid UTF-8 open-source text ({err}); Vessel refused to add it."
            ))
        })?;

        let package = build_nuvio_scraper_package(
            &manifest_val,
            &final_manifest_url,
            scraper_id,
            &raw_js,
            &final_js_url,
        )?;
        Ok(ResolvedSource::Package {
            resolved_url: final_js_url,
            package,
            badge,
        })
    }

    async fn lookup_shortcode(&self, code: &str) -> VesselResult<String> {
        let mut last_err: Option<VesselError> = None;
        for reg_url in &self.registry_urls {
            match self.try_fetch_bytes(reg_url).await {
                Ok(Some((_, bytes))) => {
                    let registry: RegistryIndex =
                        serde_json::from_slice(&bytes).map_err(|err| {
                            VesselError::InvalidInput(format!(
                                "invalid registry.json at '{reg_url}': {err}"
                            ))
                        })?;
                    if let Some(target) = registry.codes.get(code) {
                        return Ok(target.clone());
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }

        if let Some(ref shortener) = self.shortener_base_url {
            let probe_url = format!("{shortener}/{code}");
            let parsed = self.validate_url(&probe_url)?;
            let resp = self.client.get(parsed).send().await.map_err(|err| {
                VesselError::Network(format!("resolve shortcode '{code}': {err}"))
            })?;
            if resp.status().is_redirection()
                && let Some(location) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok())
            {
                return Ok(location.to_owned());
            }
        }

        if let Some(target) = builtin_shortcode(code) {
            return Ok(target.to_owned());
        }

        if let Some(err) = last_err {
            return Err(err);
        }

        Err(VesselError::NotFound(format!(
            "shortcode '{code}' not found in configured registries"
        )))
    }

    async fn probe_git_raw_bases(
        &self,
        base_urls: &[String],
        badge: SourceTrustBadge,
    ) -> VesselResult<ResolvedSource> {
        for base in base_urls {
            let repo_url = join_base_path(base, "repo.json");
            if let Some((final_url, bytes)) = self.try_fetch_bytes(&repo_url).await? {
                let repo = self.parse_and_expand_repo(&bytes, Some(base)).await?;
                return Ok(ResolvedSource::Repository {
                    resolved_url: final_url,
                    base_raw_url: Some(base.clone()),
                    repo,
                    badge,
                });
            }

            let manifest_url = join_base_path(base, "manifest.json");
            if let Some((final_url, bytes)) = self.try_fetch_bytes(&manifest_url).await? {
                if let Some(ext) = parse_external_manifest(&bytes, &final_url)? {
                    match ext {
                        ExternalManifestKind::NuvioRepo(repo) => {
                            return Ok(ResolvedSource::Repository {
                                resolved_url: final_url,
                                base_raw_url: Some(base.clone()),
                                repo: *repo,
                                badge,
                            });
                        }
                        ExternalManifestKind::StremioAddon(package) => {
                            return Ok(ResolvedSource::Package {
                                resolved_url: final_url,
                                package: *package,
                                badge,
                            });
                        }
                    }
                }
                let manifest: PluginManifest = serde_json::from_slice(&bytes).map_err(|err| {
                    VesselError::InvalidInput(format!(
                        "invalid manifest.json at '{final_url}': {err}"
                    ))
                })?;
                let package = self.fetch_open_package(base, manifest).await?;
                return Ok(ResolvedSource::Package {
                    resolved_url: final_url,
                    package,
                    badge,
                });
            }
        }

        Err(VesselError::NotFound(
            "neither repo.json nor manifest.json found on target repository branches".into(),
        ))
    }

    async fn resolve_direct_url(
        &self,
        url: &str,
        badge: SourceTrustBadge,
    ) -> VesselResult<ResolvedSource> {
        let (final_url, bytes) = self.fetch_bytes_checked(url).await?;
        if final_url.ends_with(".plugin.vessel") || bytes.starts_with(b"PK\x03\x04") {
            let package = load_plugin_package(&bytes, None)?;
            return Ok(ResolvedSource::Package {
                resolved_url: final_url,
                package,
                badge,
            });
        }

        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            check_cloudstream_closed_plugins(&bytes, &final_url)?;
            let base = dir_base_url(&final_url);
            if value.get("plugins").is_some() || value.get("pluginLists").is_some() {
                let repo = self.parse_and_expand_repo(&bytes, base.as_deref()).await?;
                return Ok(ResolvedSource::Repository {
                    resolved_url: final_url,
                    base_raw_url: base,
                    repo,
                    badge,
                });
            }
            if let Some(ext) = parse_external_manifest(&bytes, &final_url)? {
                match ext {
                    ExternalManifestKind::NuvioRepo(repo) => {
                        return Ok(ResolvedSource::Repository {
                            resolved_url: final_url,
                            base_raw_url: base,
                            repo: *repo,
                            badge,
                        });
                    }
                    ExternalManifestKind::StremioAddon(package) => {
                        return Ok(ResolvedSource::Package {
                            resolved_url: final_url,
                            package: *package,
                            badge,
                        });
                    }
                }
            }
            if value.get("entry").is_some() {
                let manifest: PluginManifest = serde_json::from_value(value).map_err(|err| {
                    VesselError::InvalidInput(format!("invalid manifest JSON: {err}"))
                })?;
                let base_url = base.ok_or_else(|| {
                    VesselError::InvalidInput(
                        "cannot resolve relative files without base URL".into(),
                    )
                })?;
                let package = self.fetch_open_package(&base_url, manifest).await?;
                return Ok(ResolvedSource::Package {
                    resolved_url: final_url,
                    package,
                    badge,
                });
            }
        }

        Err(VesselError::InvalidInput(format!(
            "target '{final_url}' is neither a .plugin.vessel package, repo.json, nor manifest.json"
        )))
    }

    async fn resolve_local(
        &self,
        path: &Path,
        badge: SourceTrustBadge,
    ) -> VesselResult<ResolvedSource> {
        let repo_file = if path.is_dir() {
            path.join("repo.json")
        } else {
            path.to_path_buf()
        };
        if repo_file.is_file() && repo_file.file_name().is_some_and(|n| n == "repo.json") {
            let bytes = fs::read(&repo_file).map_err(|err| {
                VesselError::Storage(format!("read {}: {err}", repo_file.display()))
            })?;
            let repo: RepoIndex = serde_json::from_slice(&bytes)
                .map_err(|err| VesselError::InvalidInput(format!("invalid repo.json: {err}")))?;
            return Ok(ResolvedSource::Repository {
                resolved_url: repo_file.to_string_lossy().into_owned(),
                base_raw_url: None,
                repo,
                badge,
            });
        }

        if path.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| ext == "vessel" || ext == "zip")
        {
            let bytes = fs::read(path)
                .map_err(|err| VesselError::Storage(format!("read {}: {err}", path.display())))?;
            let package = load_plugin_package(&bytes, None)?;
            return Ok(ResolvedSource::Package {
                resolved_url: path.to_string_lossy().into_owned(),
                package,
                badge,
            });
        }

        let dir = if path.file_name().is_some_and(|n| n == "manifest.json") {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let package = load_plugin_dir(dir)?;
        Ok(ResolvedSource::Package {
            resolved_url: dir.to_string_lossy().into_owned(),
            package,
            badge,
        })
    }

    async fn parse_and_expand_repo(
        &self,
        bytes: &[u8],
        base_url: Option<&str>,
    ) -> VesselResult<RepoIndex> {
        let raw_val: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|err| VesselError::InvalidInput(format!("invalid repo.json: {err}")))?;

        let mut repo: RepoIndex = if raw_val.get("schemaVersion").is_none()
            && raw_val.get("pluginLists").is_some()
        {
            // CloudStream `repo.json` omits Vessel's `schemaVersion`
            let name = raw_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("CloudStream Repository")
                .to_owned();
            let lists: Vec<String> = raw_val
                .get("pluginLists")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            RepoIndex {
                schema_version: 1,
                id: name.to_ascii_lowercase().replace(' ', "-"),
                name,
                description: raw_val
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                author: None,
                website: None,
                icon_url: raw_val
                    .get("iconUrl")
                    .or_else(|| raw_val.get("icon"))
                    .or_else(|| raw_val.get("logo"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                plugin_lists: lists,
                shared_extractors: None,
                plugins: Vec::new(),
            }
        } else {
            let parsed: RepoIndex = serde_json::from_value(raw_val)
                .map_err(|err| VesselError::InvalidInput(format!("invalid repo.json: {err}")))?;
            if parsed.schema_version != 1 {
                return Err(VesselError::InvalidInput(format!(
                    "unsupported repo.json schemaVersion: {}",
                    parsed.schema_version
                )));
            }
            parsed
        };

        // Follow `pluginLists` up to depth 1 per PLAN.md 7.1.
        if MAX_PLUGIN_LIST_DEPTH >= 1 && !repo.plugin_lists.is_empty() {
            let list_urls = std::mem::take(&mut repo.plugin_lists);
            for raw_list_url in list_urls {
                let target_url = if raw_list_url.starts_with("https://")
                    || raw_list_url.starts_with("http://")
                {
                    raw_list_url
                } else if let Some(base) = base_url {
                    join_base_path(base, &raw_list_url)
                } else {
                    continue;
                };
                if let Some((_, list_bytes)) = self.try_fetch_bytes(&target_url).await? {
                    check_cloudstream_closed_plugins(&list_bytes, &target_url)?;
                    if let Ok(sub_repo) = serde_json::from_slice::<RepoIndex>(&list_bytes) {
                        repo.plugins.extend(sub_repo.plugins);
                    }
                }
            }
        }

        if repo.icon_url.is_none() {
            repo.icon_url = repo.plugins.iter().find_map(|p| p.icon_url.clone());
        }

        Ok(repo)
    }

    /// Downloads an open directory plugin by tracing static `oxc` imports from `manifest.entry`.
    async fn fetch_open_package(
        &self,
        base_url: &str,
        manifest: PluginManifest,
    ) -> VesselResult<LoadedPackage> {
        manifest.validate()?;
        let mut files: BTreeMap<String, String> = BTreeMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        queue.push_back(manifest.entry.trim_start_matches("./").to_owned());
        for extra in &manifest.files {
            queue.push_back(extra.trim_start_matches("./").to_owned());
        }

        while let Some(stem) = queue.pop_front() {
            let (resolved_rel, text) = self.fetch_module_with_extensions(base_url, &stem).await?;
            verify_open_source_js(&text, &join_base_path(base_url, &resolved_rel))?;
            if !visited.insert(resolved_rel.clone()) {
                continue;
            }

            let imports = collect_relative_imports(&resolved_rel, &text)?;
            let parent_dir = match resolved_rel.rsplit_once('/') {
                Some((dir, _)) => dir,
                None => "",
            };
            for spec in imports {
                let normalized = normalize_rel_import(parent_dir, &spec)?;
                if !visited.contains(&normalized) {
                    queue.push_back(normalized);
                }
            }
            files.insert(resolved_rel, text);
        }

        let icon_png = self
            .try_fetch_bytes(&join_base_path(base_url, "icon.png"))
            .await?
            .map(|(_, bytes)| bytes);

        let mut hash_buf = serde_json::to_vec(&manifest).unwrap_or_default();
        for (path, text) in &files {
            hash_buf.extend_from_slice(path.as_bytes());
            hash_buf.extend_from_slice(text.as_bytes());
        }
        let sha256 = sha256_hex(&hash_buf);
        let bundle = PluginBundle::new(files)?;

        Ok(LoadedPackage {
            manifest,
            bundle,
            icon_png,
            sha256,
        })
    }

    async fn fetch_module_with_extensions(
        &self,
        base_url: &str,
        stem: &str,
    ) -> VesselResult<(String, String)> {
        for suffix in TRY_MODULE_EXTENSIONS {
            let rel = if suffix.is_empty() {
                stem.to_owned()
            } else {
                format!("{stem}{suffix}")
            };
            let url = join_base_path(base_url, &rel);
            if let Some((_, bytes)) = self.try_fetch_bytes(&url).await? {
                let text = String::from_utf8(bytes).map_err(|err| {
                    VesselError::InvalidInput(format!("non-UTF8 source at '{url}': {err}"))
                })?;
                return Ok((rel, text));
            }
        }
        Err(VesselError::NotFound(format!(
            "module '{stem}' not found under '{base_url}'"
        )))
    }

    /// Downloads an individual `RepoPluginEntry` via `download_url` (with `sha256`) or `source_path`.
    pub async fn install_repo_plugin(
        &self,
        repo_base_url: Option<&str>,
        entry: &RepoPluginEntry,
    ) -> VesselResult<LoadedPackage> {
        if let Some(ref download_url) = entry.download_url {
            let (_, bytes) = self.fetch_bytes_checked(download_url).await?;
            return load_plugin_package(&bytes, entry.sha256.as_deref());
        }

        if let Some(ref source_path) = entry.source_path {
            let target = if source_path.starts_with("gh:")
                || source_path.starts_with("gl:")
                || source_path.starts_with("https://")
                || source_path.starts_with("http://")
            {
                match self.resolve(source_path).await? {
                    ResolvedSource::Package { package, .. } => package,
                    ResolvedSource::Repository { .. } => {
                        return Err(VesselError::InvalidInput(format!(
                            "sourcePath '{source_path}' resolved to a repository instead of a plugin"
                        )));
                    }
                }
            } else {
                let base = repo_base_url.ok_or_else(|| {
                    VesselError::InvalidInput(
                        "relative sourcePath requires repository base URL".into(),
                    )
                })?;
                let plugin_base = join_base_path(base, source_path);
                let manifest_url = join_base_path(&plugin_base, "manifest.json");
                let (_, bytes) = self.fetch_bytes_checked(&manifest_url).await?;
                if let Some(ref expected) = entry.sha256 {
                    // For single-file/manifest checks when sha256 targets manifest if provided:
                    // open dirs normally omit package zip sha256, so keep manifest hash optional unless package zip exists.
                    let _ = verify_sha256(&bytes, expected);
                }
                let manifest: PluginManifest = serde_json::from_slice(&bytes).map_err(|err| {
                    VesselError::InvalidInput(format!("invalid plugin manifest.json: {err}"))
                })?;
                self.fetch_open_package(&plugin_base, manifest).await?
            };
            return Ok(target);
        }

        Err(VesselError::InvalidInput(format!(
            "repo plugin '{}' has neither downloadUrl nor sourcePath",
            entry.id
        )))
    }
}

fn normalize_rel_import(parent_dir: &str, spec: &str) -> VesselResult<String> {
    let mut stack: Vec<&str> = parent_dir.split('/').filter(|s| !s.is_empty()).collect();
    for segment in spec.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return Err(VesselError::InvalidInput(format!(
                        "import '{spec}' escapes plugin root"
                    )));
                }
            }
            part => stack.push(part),
        }
    }
    Ok(stack.join("/"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::Router;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Redirect};
    use axum::routing::get;
    use tokio::net::TcpListener;
    use vessel_domain::{ContentProvider, PluginManifest};

    use super::{
        ParsedSource, ResolvedSource, SourceResolver, SourceTrustBadge, badge_for_source,
        parse_source_input,
    };
    use crate::package::pack_plugin_package;
    use crate::ts_strip::sha256_hex;
    use crate::{DomainPolicy, HostServices, SandboxLimits, TsCompileCache};

    #[test]
    fn parses_all_source_formats_and_controls_official_badge() {
        let gh = parse_source_input("gh:falsisdev/anthology@dev/plugins/main").expect("gh");
        assert_eq!(
            gh,
            ParsedSource::GitHub {
                owner: "falsisdev".into(),
                repo: "anthology".into(),
                git_ref: Some("dev".into()),
                subpath: "plugins/main".into(),
            }
        );
        assert_eq!(badge_for_source(&gh), SourceTrustBadge::Official);

        let gh_third_party = parse_source_input("gh:community/streams").expect("gh community");
        assert_eq!(
            badge_for_source(&gh_third_party),
            SourceTrustBadge::Community
        );

        let gl = parse_source_input("gl:media/nested/repo/src").expect("gl");
        assert_eq!(
            gl,
            ParsedSource::GitLab {
                project_path: "media/nested".into(),
                git_ref: None,
                subpath: "repo/src".into(),
            }
        );

        let deep = parse_source_input("vessel://install?src=gh%3Afalsisdev%2Fanthology")
            .expect("deep link");
        assert!(matches!(
            deep,
            ParsedSource::GitHub {
                owner,
                ..
            } if owner == "falsisdev"
        ));

        assert_eq!(
            parse_source_input("anthology").expect("shortcode"),
            ParsedSource::Shortcode("anthology".into())
        );
    }

    #[tokio::test]
    async fn resolves_shortcode_repo_open_ts_imports_and_signed_vessel_zip() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock source server");
        let addr = listener.local_addr().expect("addr");
        let base_url = format!("http://{addr}");

        let zip_manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": "anthology-zip",
            "name": "Anthology Zip",
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["vod"]
        }))
        .expect("zip manifest");
        let mut zip_files = BTreeMap::new();
        zip_files.insert(
            "index.ts".into(),
            "export default { async getHomeFeed() { return { sections: [] }; } };".into(),
        );
        let zip_bytes = pack_plugin_package(&zip_manifest, &zip_files, None).expect("pack zip");
        let zip_sha = sha256_hex(&zip_bytes);

        let repo_json = serde_json::json!({
            "schemaVersion": 1,
            "id": "dev.falsis.anthology",
            "name": "Anthology Repo",
            "pluginLists": ["extra.json"],
            "plugins": [{
                "id": "anthology-zip",
                "name": "Anthology Zip",
                "version": "1.0.0",
                "capabilities": ["vod"],
                "downloadUrl": format!("{base_url}/dist/anthology.plugin.vessel"),
                "sha256": zip_sha,
            }]
        });
        let extra_repo_json = serde_json::json!({
            "schemaVersion": 1,
            "id": "dev.falsis.extra",
            "name": "Extra",
            "plugins": [{
                "id": "anthology-open",
                "name": "Anthology Open TS",
                "version": "0.1.0",
                "capabilities": ["vod"],
                "sourcePath": "open-plugin",
            }]
        });

        let open_manifest = serde_json::json!({
            "schemaVersion": 1,
            "id": "anthology",
            "name": "Anthology",
            "version": "0.1.0",
            "entry": "src/index.ts",
            "capabilities": ["vod"]
        });

        let registry_json = serde_json::json!({
            "schemaVersion": 1,
            "codes": {
                "anthology": "gh:falsisdev/anthology"
            }
        });

        let base_for_redirect = base_url.clone();
        let app = Router::new()
            .route(
                "/registry.json",
                get(move || async move { axum::Json(registry_json.clone()) }),
            )
            .route(
                "/s/anthology-go",
                get(move || async move {
                    Redirect::temporary(&format!("{base_for_redirect}/dist/anthology.plugin.vessel"))
                }),
            )
            .route(
                "/gh/falsisdev/anthology/main/repo.json",
                get(move || async move { axum::Json(repo_json.clone()) }),
            )
            .route(
                "/gh/falsisdev/anthology/main/extra.json",
                get(move || async move { axum::Json(extra_repo_json.clone()) }),
            )
            .route(
                "/gh/falsisdev/anthology/main/open-plugin/manifest.json",
                get(move || async move { axum::Json(open_manifest.clone()) }),
            )
            .route(
                "/gh/falsisdev/anthology/main/open-plugin/src/index.ts",
                get(|| async {
                    (
                        StatusCode::OK,
                        r#"
                        import { definePlugin } from "vessel";
                        import { sectionTitle } from "./helpers";
                        export default definePlugin({
                            async getHomeFeed() {
                                return { sections: [{ id: "main", title: sectionTitle(), items: [] }] };
                            }
                        });
                        "#,
                    )
                }),
            )
            .route(
                "/gh/falsisdev/anthology/main/open-plugin/src/helpers.ts",
                get(|| async { (StatusCode::OK, "export const sectionTitle = (): string => 'Trending';") }),
            )
            .route(
                "/dist/anthology.plugin.vessel",
                get(move || async move { (StatusCode::OK, zip_bytes.clone()).into_response() }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let resolver = SourceResolver::new()
            .with_registries([format!("{base_url}/registry.json")])
            .with_shortener_base(format!("{base_url}/s"))
            .with_test_endpoints(format!("{base_url}/gh"), format!("{base_url}/gl"));

        // 1. Shortcode -> registry.json -> gh:falsisdev/anthology -> Official RepoIndex + pluginLists depth 1.
        let resolved = resolver
            .resolve("anthology")
            .await
            .expect("resolve shortcode");
        let (repo_base, repo) = match resolved {
            ResolvedSource::Repository {
                base_raw_url,
                repo,
                badge,
                ..
            } => {
                assert_eq!(badge, SourceTrustBadge::Official);
                assert_eq!(repo.plugins.len(), 2);
                (base_raw_url, repo)
            }
            other => panic!("expected Repository, got {other:?}"),
        };

        // 2. Install signed .plugin.vessel archive from repo entry.
        let installed_zip = resolver
            .install_repo_plugin(repo_base.as_deref(), &repo.plugins[0])
            .await
            .expect("install signed .plugin.vessel");
        assert_eq!(installed_zip.manifest.id.as_str(), "anthology-zip");

        // 3. Open TS sub-folder plugin resolved recursively via oxc import graph & loaded into QuickJS.
        let installed_open = resolver
            .install_repo_plugin(repo_base.as_deref(), &repo.plugins[1])
            .await
            .expect("install open TS plugin");
        assert!(installed_open.bundle.get("src/helpers.ts").is_some());

        // 4. Shortener HTTP 302 redirect resolves directly to .plugin.vessel package.
        let via_redirect = resolver
            .resolve("anthology-go")
            .await
            .expect("resolve 302 shortener");
        assert!(matches!(via_redirect, ResolvedSource::Package { .. }));

        // Verify the resolved open TS package actually executes inside QuickJsContentProvider.
        let storage = crate::provider::provider_test_storage();
        let host = std::sync::Arc::new(
            HostServices::new(
                installed_open.manifest.id.clone(),
                std::sync::Arc::new(DomainPolicy::new(
                    installed_open.manifest.id.clone(),
                    vec![],
                    false,
                )),
                storage,
                None,
                std::sync::Arc::new(TsCompileCache::new()),
            )
            .expect("host services"),
        );
        let provider = crate::QuickJsContentProvider::load(
            installed_open.manifest,
            std::sync::Arc::new(installed_open.bundle),
            host,
            SandboxLimits::default(),
        )
        .await
        .expect("load resolved TS package into QuickJS");

        let feed = provider.home_feed(None).await.expect("home_feed");
        assert_eq!(feed.sections[0].title, "Trending");
        provider.shutdown().await;
    }

    #[tokio::test]
    async fn resolves_builtin_official_shortcodes_anthology_and_tr() {
        let resolver = SourceResolver::new();
        assert_eq!(
            resolver
                .lookup_shortcode("anthology")
                .await
                .expect("anthology shortcode"),
            "gh:falsisdev/anthology"
        );
        assert_eq!(
            resolver.lookup_shortcode("tr").await.expect("tr shortcode"),
            "gh:falsisdev/anthology"
        );
        assert!(resolver.lookup_shortcode("unknown-code").await.is_err());

        let repo_registry_raw = include_str!("../../../registry.json");
        let parsed: vessel_domain::RegistryIndex =
            serde_json::from_str(repo_registry_raw).expect("parse root registry.json");
        assert_eq!(
            parsed.codes.get("anthology").map(String::as_str),
            Some("gh:falsisdev/anthology")
        );
        assert_eq!(
            parsed.codes.get("tr").map(String::as_str),
            Some("gh:falsisdev/anthology")
        );
    }
}
