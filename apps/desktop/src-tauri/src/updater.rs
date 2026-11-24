//! GitHub Releases update checker, release notes parser, and installer downloader.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::commands::AppState;

const GITHUB_OWNER: &str = "falsisdev";
const GITHUB_REPO: &str = "vessel";
pub const AUTO_CHECK_UPDATES_KEY: &str = "auto_check_updates";
pub const SKIPPED_UPDATE_VERSION_KEY: &str = "skipped_update_version";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfoDto {
    pub current_version: String,
    pub latest_version: String,
    pub tag_name: String,
    pub update_available: bool,
    pub release_name: Option<String>,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub asset_size_bytes: Option<u64>,
    pub download_url: Option<String>,
    pub auto_check_enabled: bool,
    pub skipped_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesDto {
    pub auto_check_enabled: bool,
    pub skipped_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgressPayload {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

fn normalize_version(raw: &str) -> &str {
    raw.trim().trim_start_matches(['v', 'V'])
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSemver {
    major: u64,
    minor: u64,
    patch: u64,
    is_prerelease: bool,
    pre: String,
}

fn parse_semver(raw: &str) -> Option<ParsedSemver> {
    let cleaned = normalize_version(raw);
    let cleaned = cleaned.split('+').next().unwrap_or(cleaned);
    let (core, pre_part) = match cleaned.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (cleaned, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Some(ParsedSemver {
        major,
        minor,
        patch,
        is_prerelease: pre_part.is_some(),
        pre: pre_part.unwrap_or_default().to_ascii_lowercase(),
    })
}

fn compare_pre_segments(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_parts = a.split('.').filter(|s| !s.is_empty());
    let mut b_parts = b.split('.').filter(|s| !s.is_empty());
    loop {
        match (a_parts.next(), b_parts.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ap), Some(bp)) => {
                let ord = match (ap.parse::<u64>(), bp.parse::<u64>()) {
                    (Ok(an), Ok(bn)) => an.cmp(&bn),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => ap.cmp(bp),
                };
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

fn compare_semver(a: &ParsedSemver, b: &ParsedSemver) -> std::cmp::Ordering {
    match (a.major, a.minor, a.patch).cmp(&(b.major, b.minor, b.patch)) {
        std::cmp::Ordering::Equal => match (a.is_prerelease, b.is_prerelease) {
            (false, false) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) => compare_pre_segments(&a.pre, &b.pre),
        },
        non_eq => non_eq,
    }
}

fn is_newer_version(current: &str, candidate: &str) -> bool {
    let (Some(cur), Some(cand)) = (parse_semver(current), parse_semver(candidate)) else {
        return normalize_version(candidate) != normalize_version(current)
            && !normalize_version(candidate).is_empty();
    };
    compare_semver(&cand, &cur) == std::cmp::Ordering::Greater
}

fn select_best_asset<'a>(
    assets: &'a [GithubAsset],
    target_os: &str,
    target_arch: &str,
) -> Option<&'a GithubAsset> {
    let by_score = |asset: &GithubAsset| -> i32 {
        let lower = asset.name.to_ascii_lowercase();
        match target_os {
            "macos" => {
                if !lower.ends_with(".dmg") {
                    return -1;
                }
                let matches_arch = (target_arch == "aarch64"
                    && (lower.contains("aarch64") || lower.contains("arm64")))
                    || (target_arch == "x86_64"
                        && (lower.contains("x64") || lower.contains("x86_64")));
                if matches_arch {
                    100
                } else if lower.contains("universal") {
                    80
                } else {
                    40
                }
            }
            "windows" => {
                if lower.ends_with("-setup.exe") {
                    100
                } else if lower.ends_with(".exe") {
                    90
                } else if lower.ends_with(".msi") {
                    70
                } else {
                    -1
                }
            }
            "linux" => {
                let mentions_x64 =
                    lower.contains("amd64") || lower.contains("x86_64") || lower.contains("x64");
                let mentions_arm = lower.contains("arm64") || lower.contains("aarch64");
                if target_arch == "x86_64" && mentions_arm && !mentions_x64 {
                    return -1;
                }
                if target_arch == "aarch64" && mentions_x64 && !mentions_arm {
                    return -1;
                }
                let base = if lower.ends_with(".appimage") {
                    100
                } else if lower.ends_with(".deb") {
                    80
                } else if lower.ends_with(".rpm") {
                    60
                } else if lower.ends_with(".flatpak") {
                    40
                } else {
                    return -1;
                };
                let arch_bonus = if (target_arch == "x86_64" && mentions_x64)
                    || (target_arch == "aarch64" && mentions_arm)
                {
                    10
                } else {
                    0
                };
                base + arch_bonus
            }
            _ => -1,
        }
    };

    assets
        .iter()
        .filter(|a| by_score(a) >= 0)
        .max_by_key(|a| by_score(a))
}

fn is_allowed_github_download_host(raw_url: &str) -> Result<Url, String> {
    let parsed = Url::parse(raw_url)
        .map_err(|err| format!("[VesselInvalidInputError]: invalid update asset URL: {err}"))?;
    if parsed.scheme() != "https" {
        return Err("[VesselSecurityError]: update downloads require HTTPS".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "[VesselSecurityError]: missing download host".to_string())?
        .to_ascii_lowercase();
    let allowed = host == "github.com"
        || host == "objects.githubusercontent.com"
        || host == "release-assets.githubusercontent.com"
        || host.ends_with(".githubusercontent.com");
    if !allowed {
        return Err(format!(
            "[VesselDomainBlockedError]: refused update download from untrusted host '{host}'"
        ));
    }
    Ok(parsed)
}

fn sanitize_filename(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("vessel-update");
    let clean: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.is_empty() || clean == "." || clean == ".." {
        "vessel-update.bin".into()
    } else {
        clean
    }
}

#[tauri::command]
pub async fn get_update_preferences(
    state: State<'_, AppState>,
) -> Result<UpdatePreferencesDto, String> {
    let auto_check_enabled = state
        .engine
        .storage()
        .get_json_setting::<bool>(AUTO_CHECK_UPDATES_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or(true);
    let skipped_version = state
        .engine
        .storage()
        .get_json_setting::<String>(SKIPPED_UPDATE_VERSION_KEY)
        .await
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty());

    Ok(UpdatePreferencesDto {
        auto_check_enabled,
        skipped_version,
    })
}

#[tauri::command]
pub async fn set_auto_check_updates(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .engine
        .storage()
        .set_json_setting(AUTO_CHECK_UPDATES_KEY, &enabled)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn skip_app_update_version(
    state: State<'_, AppState>,
    version: Option<String>,
) -> Result<(), String> {
    let cleaned = version.unwrap_or_default();
    state
        .engine
        .storage()
        .set_json_setting(SKIPPED_UPDATE_VERSION_KEY, &cleaned)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn check_app_update(state: State<'_, AppState>) -> Result<AppUpdateInfoDto, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_owned();
    let prefs = get_update_preferences(state).await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent(format!("Vessel-Desktop/{current_version}"))
        .build()
        .map_err(|err| format!("[VesselNetworkError]: build updater client: {err}"))?;

    let cur_is_prerelease = parse_semver(&current_version).is_some_and(|v| v.is_prerelease);
    let list_url =
        format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases?per_page=20");
    let resp = client
        .get(&list_url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|err| format!("[VesselNetworkError]: check GitHub releases failed: {err}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "[VesselNetworkError]: GitHub releases returned HTTP {}",
            resp.status()
        ));
    }

    let releases: Vec<GithubRelease> = resp
        .json()
        .await
        .map_err(|err| format!("[VesselParseError]: decode GitHub releases: {err}"))?;

    let pick_highest = |candidates: Vec<GithubRelease>| -> Option<GithubRelease> {
        candidates.into_iter().max_by(|a, b| {
            match (parse_semver(&a.tag_name), parse_semver(&b.tag_name)) {
                (Some(sa), Some(sb)) => compare_semver(&sa, &sb),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => a.published_at.cmp(&b.published_at),
            }
        })
    };

    let stable_candidates: Vec<GithubRelease> = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .map(|r| GithubRelease {
            tag_name: r.tag_name.clone(),
            name: r.name.clone(),
            body: r.body.clone(),
            published_at: r.published_at.clone(),
            html_url: r.html_url.clone(),
            draft: r.draft,
            prerelease: r.prerelease,
            assets: r
                .assets
                .iter()
                .map(|a| GithubAsset {
                    name: a.name.clone(),
                    size: a.size,
                    browser_download_url: a.browser_download_url.clone(),
                })
                .collect(),
        })
        .collect();

    let all_candidates: Vec<GithubRelease> = releases.into_iter().filter(|r| !r.draft).collect();

    let release: Option<GithubRelease> = if cur_is_prerelease {
        pick_highest(all_candidates)
    } else if !stable_candidates.is_empty() {
        pick_highest(stable_candidates)
    } else {
        pick_highest(all_candidates)
    };

    let Some(rel) = release else {
        return Ok(AppUpdateInfoDto {
            current_version: current_version.clone(),
            latest_version: current_version,
            tag_name: format!("v{}", env!("CARGO_PKG_VERSION")),
            update_available: false,
            release_name: None,
            release_notes: String::new(),
            published_at: None,
            html_url: format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases"),
            asset_name: None,
            asset_size_bytes: None,
            download_url: None,
            auto_check_enabled: prefs.auto_check_enabled,
            skipped_version: prefs.skipped_version,
        });
    };

    let latest_version = normalize_version(&rel.tag_name).to_owned();
    let update_available = is_newer_version(&current_version, &latest_version);
    let in_flatpak = std::env::var_os("FLATPAK_ID").is_some();
    let best_asset = if in_flatpak {
        None
    } else {
        select_best_asset(&rel.assets, std::env::consts::OS, std::env::consts::ARCH)
    };

    Ok(AppUpdateInfoDto {
        current_version,
        latest_version,
        tag_name: rel.tag_name,
        update_available,
        release_name: rel.name,
        release_notes: rel.body.unwrap_or_default(),
        published_at: rel.published_at,
        html_url: rel.html_url,
        asset_name: best_asset.map(|a| a.name.clone()),
        asset_size_bytes: best_asset.map(|a| a.size),
        download_url: best_asset.map(|a| a.browser_download_url.clone()),
        auto_check_enabled: prefs.auto_check_enabled,
        skipped_version: prefs.skipped_version,
    })
}

#[tauri::command]
pub async fn download_and_open_update(
    app: AppHandle,
    download_url: String,
    file_name: String,
) -> Result<String, String> {
    let url = is_allowed_github_download_host(&download_url)?;
    let safe_name = sanitize_filename(&file_name);

    let dest_dir: PathBuf = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_cache_dir())
        .unwrap_or_else(|_| std::env::temp_dir());
    std::fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("[VesselStorageError]: create update folder: {err}"))?;
    let dest_path = dest_dir.join(&safe_name);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(format!("Vessel-Desktop/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("[VesselNetworkError]: create download client: {err}"))?;

    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("[VesselNetworkError]: download update failed: {err}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "[VesselNetworkError]: update download returned HTTP {}",
            resp.status()
        ));
    }

    let total_bytes = resp.content_length();
    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|err| format!("[VesselStorageError]: create update file: {err}"))?;

    let mut downloaded_bytes: u64 = 0;
    let mut last_percent: Option<u8> = None;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|err| format!("[VesselNetworkError]: read update chunk: {err}"))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|err| format!("[VesselStorageError]: write update file: {err}"))?;
        downloaded_bytes += chunk.len() as u64;
        let percent = total_bytes.map(|total| {
            downloaded_bytes
                .saturating_mul(100)
                .checked_div(total)
                .map_or(100, |p| p.min(100) as u8)
        });
        if percent != last_percent || downloaded_bytes == total_bytes.unwrap_or(0) {
            last_percent = percent;
            let _ = app.emit(
                "vessel://update-progress",
                UpdateDownloadProgressPayload {
                    file_name: safe_name.clone(),
                    downloaded_bytes,
                    total_bytes,
                    percent,
                },
            );
        }
    }
    file.flush()
        .await
        .map_err(|err| format!("[VesselStorageError]: flush update file: {err}"))?;

    open_installer_path(&dest_path)?;
    Ok(dest_path.to_string_lossy().into_owned())
}

#[cfg(target_os = "macos")]
fn open_installer_path(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|err| format!("[VesselProcessError]: launch installer: {err}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_installer_path(path: &std::path::Path) -> Result<(), String> {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if lower.ends_with(".msi") {
        std::process::Command::new("msiexec")
            .arg("/i")
            .arg(path)
            .spawn()
            .map_err(|err| format!("[VesselProcessError]: launch MSI installer: {err}"))?;
    } else {
        std::process::Command::new(path)
            .spawn()
            .map_err(|err| format!("[VesselProcessError]: launch setup installer: {err}"))?;
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_installer_path(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|err| format!("[VesselProcessError]: open update package: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        GithubAsset, is_allowed_github_download_host, is_newer_version, select_best_asset,
    };

    #[test]
    fn compares_semver_and_prereleases_cleanly() {
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.2.0-beta.1", "0.2.0"));
        assert!(is_newer_version("0.1.1-alpha.1", "v0.1.1-alpha.2"));
        assert!(is_newer_version("0.1.1-alpha.2", "v0.1.1-alpha.10"));
        assert!(!is_newer_version("0.1.1-alpha.10", "v0.1.1-alpha.2"));
        assert!(!is_newer_version("0.2.0", "0.2.0-beta.1"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("1.0.0", "v0.9.9"));
    }

    #[test]
    fn picks_platform_installer_assets() {
        let assets = vec![
            GithubAsset {
                name: "Vessel_0.2.0_x64.dmg".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/x64.dmg".into(),
            },
            GithubAsset {
                name: "Vessel_0.2.0_aarch64.dmg".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/aarch64.dmg".into(),
            },
            GithubAsset {
                name: "Vessel_0.2.0_x64-setup.exe".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/setup.exe".into(),
            },
            GithubAsset {
                name: "Vessel_0.2.0_amd64.AppImage".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/amd64.AppImage".into(),
            },
            GithubAsset {
                name: "Vessel_0.2.0_amd64.deb".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/amd64.deb".into(),
            },
            GithubAsset {
                name: "Vessel_0.2.0_amd64.flatpak".into(),
                size: 100,
                browser_download_url: "https://github.com/a/b/amd64.flatpak".into(),
            },
        ];

        assert_eq!(
            select_best_asset(&assets, "macos", "aarch64").map(|a| a.name.as_str()),
            Some("Vessel_0.2.0_aarch64.dmg")
        );
        assert_eq!(
            select_best_asset(&assets, "macos", "x86_64").map(|a| a.name.as_str()),
            Some("Vessel_0.2.0_x64.dmg")
        );
        assert_eq!(
            select_best_asset(&assets, "windows", "x86_64").map(|a| a.name.as_str()),
            Some("Vessel_0.2.0_x64-setup.exe")
        );
        assert_eq!(
            select_best_asset(&assets, "linux", "x86_64").map(|a| a.name.as_str()),
            Some("Vessel_0.2.0_amd64.AppImage")
        );
    }

    #[test]
    fn restricts_update_downloads_to_github_hosts() {
        assert!(
            is_allowed_github_download_host(
                "https://github.com/falsisdev/vessel/releases/download/v0.2.0/Vessel.dmg"
            )
            .is_ok()
        );
        assert!(
            is_allowed_github_download_host("https://objects.githubusercontent.com/file.dmg")
                .is_ok()
        );
        assert!(is_allowed_github_download_host("https://evil.example.com/Vessel.dmg").is_err());
        assert!(is_allowed_github_download_host("http://github.com/Vessel.dmg").is_err());
    }
}
