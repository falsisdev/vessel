//! Tauri IPC command handlers delegating cleanly to `VesselEngine`.

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::task::JoinSet;
use vessel_domain::{
    HomeSection, LibraryItem, LibraryStatus, MediaDetail, MediaRef, MediaType, PlaybackProgress,
    PluginId, PluginManifest, RepoIndex, SecretStore, StreamCandidate, StreamResult, SubtitleTrack,
    ThemeColors, ThemeSpec, UiPreferences, VesselPaths, VesselResult,
};
use vessel_engine::{
    DownloadRequest, EpgNowNext, IptvKind, IptvProfile, PlayCandidateRequest, TMDB_API_KEY_SECRET,
    VesselEngine,
};
use vessel_plugin_rt::{ResolvedSource, SourceTrustBadge};
use vessel_storage::{
    BackupImportMode, ContinueWatchingEntry, DownloadKind, StoredDownload, StoredIptvChannel,
    StoredIptvProfile,
};

/// Shared Tauri managed state wrapping the pure-Rust engine and path layout.
pub struct AppState {
    pub engine: Arc<VesselEngine>,
    pub paths: VesselPaths,
}

fn map_err<T>(res: VesselResult<T>) -> Result<T, String> {
    res.map_err(|err| err.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfoDto {
    pub root_dir: String,
    pub downloads_dir: String,
    pub plugins_dir: String,
    pub themes_dir: String,
    pub locales_dir: String,
    pub backups_dir: String,
    pub proxy_base_url: String,
    pub always_proxy: bool,
    pub metadata_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginListEntryDto {
    pub manifest: PluginManifest,
    pub repo_id: Option<String>,
    pub source_type: String,
    pub local_path: String,
    pub sha256: Option<String>,
    pub enabled: bool,
    pub allow_dynamic_domains: bool,
    pub allowed_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolvedSourceDto {
    #[serde(rename_all = "camelCase")]
    Repository {
        resolved_url: String,
        badge: String,
        repo: RepoIndex,
    },
    #[serde(rename_all = "camelCase")]
    Package {
        resolved_url: String,
        badge: String,
        manifest: PluginManifest,
        sha256: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeShelfResultDto {
    pub plugin_id: String,
    pub plugin_name: String,
    pub sections: Vec<HomeSection>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResultDto {
    pub plugin_id: String,
    pub badge: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayCandidatePayload {
    pub plugin_id: String,
    pub native_id: String,
    pub canonical_id: Option<String>,
    pub season: u32,
    pub episode: u32,
    pub absolute_episode: Option<u32>,
    pub media_type: MediaType,
    pub candidate: StreamCandidate,
    #[serde(default)]
    pub extra_subtitles: Vec<SubtitleTrack>,
    pub start_position_ms: Option<u64>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueDownloadPayload {
    pub plugin_id: String,
    pub native_id: String,
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub kind: DownloadKind,
    pub source_url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub subtitles: Vec<SubtitleTrack>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertIptvProfilePayload {
    pub id: String,
    pub name: String,
    pub kind: IptvKind,
    pub server_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub epg_url: Option<String>,
    pub user_agent: Option<String>,
}

fn badge_label(badge: SourceTrustBadge) -> &'static str {
    match badge {
        SourceTrustBadge::Official => "official",
        SourceTrustBadge::Community => "community",
    }
}

fn builtin_themes() -> Vec<ThemeSpec> {
    vec![
        ThemeSpec::catppuccin_mocha(),
        ThemeSpec {
            schema_version: 1,
            id: "catppuccin-latte".into(),
            name: "Catppuccin Latte".into(),
            author: "Vessel".into(),
            is_dark: false,
            colors: ThemeColors {
                bg_base: "#eff1f5".into(),
                bg_surface: "#e6e9ef".into(),
                bg_elevated: "#ccd0da".into(),
                text_primary: "#4c4f69".into(),
                text_muted: "#6c6f85".into(),
                accent: "#8839ef".into(),
                accent_secondary: "#1e66f5".into(),
                border: "#bcc0cc".into(),
                success: "#40a02b".into(),
                warning: "#df8e1d".into(),
                danger: "#d20f39".into(),
            },
        },
        ThemeSpec {
            schema_version: 1,
            id: "oled-black".into(),
            name: "OLED Pitch Black".into(),
            author: "Vessel".into(),
            is_dark: true,
            colors: ThemeColors {
                bg_base: "#000000".into(),
                bg_surface: "#0a0a0f".into(),
                bg_elevated: "#181822".into(),
                text_primary: "#e6e8f0".into(),
                text_muted: "#9499ad".into(),
                accent: "#cba6f7".into(),
                accent_secondary: "#74c7ec".into(),
                border: "#252636".into(),
                success: "#a6e3a1".into(),
                warning: "#f9e2af".into(),
                danger: "#f38ba8".into(),
            },
        },
        ThemeSpec {
            schema_version: 1,
            id: "nord".into(),
            name: "Nord".into(),
            author: "Vessel".into(),
            is_dark: true,
            colors: ThemeColors {
                bg_base: "#2e3440".into(),
                bg_surface: "#272c36".into(),
                bg_elevated: "#3b4252".into(),
                text_primary: "#eceff4".into(),
                text_muted: "#aeb7c8".into(),
                accent: "#88c0d0".into(),
                accent_secondary: "#81a1c1".into(),
                border: "#4c566a".into(),
                success: "#a3be8c".into(),
                warning: "#ebcb8b".into(),
                danger: "#bf616a".into(),
            },
        },
        ThemeSpec {
            schema_version: 1,
            id: "tokyo-night".into(),
            name: "Tokyo Night".into(),
            author: "Vessel".into(),
            is_dark: true,
            colors: ThemeColors {
                bg_base: "#1a1b26".into(),
                bg_surface: "#16161e".into(),
                bg_elevated: "#24283b".into(),
                text_primary: "#c0caf5".into(),
                text_muted: "#787c99".into(),
                accent: "#bb9af7".into(),
                accent_secondary: "#7aa2f7".into(),
                border: "#3b4261".into(),
                success: "#9ece6a".into(),
                warning: "#e0af68".into(),
                danger: "#f7768e".into(),
            },
        },
    ]
}

#[tauri::command]
pub async fn get_engine_info(state: State<'_, AppState>) -> Result<EngineInfoDto, String> {
    Ok(EngineInfoDto {
        root_dir: state.paths.root().to_string_lossy().into_owned(),
        downloads_dir: state.engine.downloads_dir().to_string_lossy().into_owned(),
        plugins_dir: state.paths.plugins_dir().to_string_lossy().into_owned(),
        themes_dir: state.paths.themes_dir().to_string_lossy().into_owned(),
        locales_dir: state.paths.locales_dir().to_string_lossy().into_owned(),
        backups_dir: state.paths.backups_dir().to_string_lossy().into_owned(),
        proxy_base_url: state.engine.proxy().base_url(),
        always_proxy: state.engine.always_proxy(),
        metadata_enabled: state.engine.metadata_enabled(),
    })
}

#[tauri::command]
pub async fn set_download_directory(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<String, String> {
    let trimmed = path.as_deref().map(str::trim).unwrap_or("");
    let target = if trimmed.is_empty() {
        state.paths.downloads_dir()
    } else {
        std::path::PathBuf::from(trimmed)
    };
    map_err(state.engine.set_downloads_dir(target.clone()))?;
    let persisted = if trimmed.is_empty() {
        String::new()
    } else {
        target.to_string_lossy().into_owned()
    };
    map_err(
        state
            .engine
            .storage()
            .set_json_setting("custom_download_dir", &persisted)
            .await,
    )?;
    Ok(state.engine.downloads_dir().to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn pick_download_directory() -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let output = tokio::process::Command::new("osascript")
            .args([
                "-e",
                "POSIX path of (choose folder with prompt \"Select Vessel Download Directory\")",
            ])
            .output()
            .await
            .map_err(|err| format!("Failed to open folder picker: {err}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let chosen = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if chosen.is_empty() {
            Ok(None)
        } else {
            Ok(Some(chosen.trim_end_matches('/').to_owned()))
        }
    }
    #[cfg(target_os = "windows")]
    {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.FolderBrowserDialog
$dlg.Description = 'Select Vessel Download Directory'
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dlg.SelectedPath
}
"#;
        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .await
            .map_err(|err| format!("Failed to open folder picker: {err}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let chosen = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if chosen.is_empty() {
            Ok(None)
        } else {
            Ok(Some(chosen))
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = tokio::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--directory",
                "--title=Select Vessel Download Directory",
            ])
            .output()
            .await
            .map_err(|err| format!("Failed to open folder picker: {err}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let chosen = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if chosen.is_empty() {
            Ok(None)
        } else {
            Ok(Some(chosen))
        }
    }
}

#[tauri::command]
pub async fn set_always_proxy(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.engine.set_always_proxy(enabled);
    map_err(
        state
            .engine
            .storage()
            .set_json_setting("always_proxy", &enabled)
            .await,
    )
}

#[tauri::command]
pub async fn set_metadata_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.engine.set_metadata_enabled(enabled);
    map_err(
        state
            .engine
            .storage()
            .set_json_setting("metadata_enabled", &enabled)
            .await,
    )
}

#[tauri::command]
pub async fn set_tmdb_api_key(state: State<'_, AppState>, api_key: String) -> Result<(), String> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        map_err(state.engine.secrets().delete(TMDB_API_KEY_SECRET).await)
    } else {
        map_err(
            state
                .engine
                .secrets()
                .set(TMDB_API_KEY_SECRET, trimmed)
                .await,
        )
    }
}

#[tauri::command]
pub async fn get_ui_preferences(state: State<'_, AppState>) -> Result<UiPreferences, String> {
    map_err(state.engine.get_ui_preferences().await)
}

#[tauri::command]
pub async fn set_ui_preferences(
    state: State<'_, AppState>,
    prefs: UiPreferences,
) -> Result<(), String> {
    map_err(state.engine.set_ui_preferences(&prefs).await)
}

#[tauri::command]
pub async fn list_themes(state: State<'_, AppState>) -> Result<Vec<ThemeSpec>, String> {
    let mut themes = builtin_themes();
    let themes_dir = state.paths.themes_dir();
    if let Ok(entries) = fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.ends_with(".theme.vessel"))
                && let Ok(raw) = fs::read_to_string(&path)
                && let Ok(spec) = serde_json::from_str::<ThemeSpec>(&raw)
                && spec.validate().is_ok()
            {
                if let Some(slot) = themes.iter_mut().find(|t| t.id == spec.id) {
                    *slot = spec;
                } else {
                    themes.push(spec);
                }
            }
        }
    }
    Ok(themes)
}

#[tauri::command]
pub async fn import_theme_json(
    state: State<'_, AppState>,
    json: String,
) -> Result<ThemeSpec, String> {
    let spec: ThemeSpec =
        serde_json::from_str(&json).map_err(|err| format!("[VesselInvalidInputError]: {err}"))?;
    map_err(spec.validate())?;
    let themes_dir = state.paths.themes_dir();
    fs::create_dir_all(&themes_dir)
        .map_err(|err| format!("[VesselStorageError]: create themes dir: {err}"))?;
    let target = themes_dir.join(format!("{}.theme.vessel", spec.id.trim()));
    fs::write(&target, json.as_bytes())
        .map_err(|err| format!("[VesselStorageError]: write theme file: {err}"))?;
    Ok(spec)
}

#[tauri::command]
pub async fn list_custom_locales(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut codes = Vec::new();
    let locales_dir = state.paths.locales_dir();
    if let Ok(entries) = fs::read_dir(&locales_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && let Some(code) = name.strip_suffix(".json")
            {
                codes.push(code.to_string());
            }
        }
    }
    codes.sort();
    Ok(codes)
}

#[tauri::command]
pub async fn load_custom_locale(
    state: State<'_, AppState>,
    lang: String,
) -> Result<Option<BTreeMap<String, serde_json::Value>>, String> {
    let safe_lang: String = lang
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let path = state.paths.locales_dir().join(format!("{safe_lang}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|err| {
        format!(
            "[VesselStorageError]: read locale '{}': {err}",
            path.display()
        )
    })?;
    let parsed: BTreeMap<String, serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|err| format!("[VesselInvalidInputError]: invalid locale JSON: {err}"))?;
    Ok(Some(parsed))
}

#[tauri::command]
pub async fn list_plugins(state: State<'_, AppState>) -> Result<Vec<PluginListEntryDto>, String> {
    let stored = map_err(state.engine.list_plugins().await)?;
    let mut out = Vec::with_capacity(stored.len());
    for item in stored {
        let allowed_domains = map_err(state.engine.list_allowed_domains(&item.manifest.id).await)?;
        out.push(PluginListEntryDto {
            manifest: item.manifest,
            repo_id: item.repo_id,
            source_type: item.source_type,
            local_path: item.local_path,
            sha256: item.sha256,
            enabled: item.enabled,
            allow_dynamic_domains: item.allow_dynamic_domains,
            allowed_domains,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn resolve_source(
    state: State<'_, AppState>,
    raw_input: String,
) -> Result<ResolvedSourceDto, String> {
    let resolved = map_err(state.engine.resolve_source(&raw_input).await)?;
    Ok(match resolved {
        ResolvedSource::Repository {
            resolved_url,
            repo,
            badge,
            ..
        } => ResolvedSourceDto::Repository {
            resolved_url,
            badge: badge_label(badge).into(),
            repo,
        },
        ResolvedSource::Package {
            resolved_url,
            package,
            badge,
        } => ResolvedSourceDto::Package {
            resolved_url,
            badge: badge_label(badge).into(),
            manifest: package.manifest,
            sha256: package.sha256,
        },
    })
}

#[tauri::command]
pub async fn install_source(
    state: State<'_, AppState>,
    raw_input: String,
    repo_id: Option<String>,
) -> Result<InstallResultDto, String> {
    let plugins_dir = state.paths.plugins_dir();
    let (id, badge) = map_err(
        state
            .engine
            .install_package_persisted_with_repo(&plugins_dir, &raw_input, repo_id)
            .await,
    )?;
    Ok(InstallResultDto {
        plugin_id: id.to_string(),
        badge: badge_label(badge).into(),
    })
}

#[tauri::command]
pub async fn set_plugin_enabled(
    state: State<'_, AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.set_plugin_enabled(&pid, enabled).await)
}

#[tauri::command]
pub async fn uninstall_plugin(state: State<'_, AppState>, plugin_id: String) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.uninstall_plugin(&pid).await)
}

#[tauri::command]
pub async fn grant_plugin_domain(
    state: State<'_, AppState>,
    plugin_id: String,
    domain: String,
    permanent: bool,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(
        state
            .engine
            .grant_plugin_domain(&pid, &domain, permanent)
            .await,
    )
}

#[tauri::command]
pub async fn get_plugin_settings(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.get_plugin_settings(&pid).await)
}

#[tauri::command]
pub async fn set_plugin_settings(
    state: State<'_, AppState>,
    plugin_id: String,
    values: BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.set_plugin_settings(&pid, values).await)
}

#[tauri::command]
pub async fn get_home_feeds(
    state: State<'_, AppState>,
    plugin_ids: Option<Vec<String>>,
) -> Result<Vec<HomeShelfResultDto>, String> {
    let plugins = map_err(state.engine.list_plugins().await)?;
    let filter_set: Option<std::collections::HashSet<String>> =
        plugin_ids.map(|ids| ids.into_iter().collect());
    // Cap concurrent home scrapes to 4 so the 6-slot QuickJS LRU pool never thrashes evictions.
    let sem = Arc::new(tokio::sync::Semaphore::new(4));
    let mut set = JoinSet::new();

    for stored in plugins.into_iter().filter(|p| {
        p.enabled
            && filter_set
                .as_ref()
                .is_none_or(|allowed| allowed.contains(p.manifest.id.as_str()))
    }) {
        let engine = Arc::clone(&state.engine);
        let sem = Arc::clone(&sem);
        set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            let pid = stored.manifest.id.clone();
            let pname = stored.manifest.name.clone();
            match engine.home_feed(&pid, None).await {
                Ok(feed) => HomeShelfResultDto {
                    plugin_id: pid.to_string(),
                    plugin_name: pname,
                    sections: feed.sections,
                    error: None,
                },
                Err(err) => HomeShelfResultDto {
                    plugin_id: pid.to_string(),
                    plugin_name: pname,
                    sections: Vec::new(),
                    error: Some(err.to_string()),
                },
            }
        });
    }

    let mut shelves = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(shelf) = res {
            shelves.push(shelf);
        }
    }
    shelves.sort_by(|a, b| a.plugin_name.cmp(&b.plugin_name));
    Ok(shelves)
}

#[tauri::command]
pub async fn search_all(
    state: State<'_, AppState>,
    query: String,
    filters: Option<BTreeMap<String, String>>,
    page: Option<u32>,
    plugin_ids: Option<Vec<String>>,
) -> Result<String, String> {
    let parsed_plugins = match plugin_ids {
        Some(ids) if !ids.is_empty() => Some(
            ids.into_iter()
                .map(PluginId::new)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?,
        ),
        _ => None,
    };
    let handle = state
        .engine
        .search_selected(
            &query,
            filters.unwrap_or_default(),
            page.unwrap_or(1),
            parsed_plugins,
        )
        .await;
    Ok(handle.search_id)
}

#[tauri::command]
pub async fn cancel_search(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.cancel_active_search().await;
    Ok(())
}

#[tauri::command]
pub async fn get_details_with_enrichment(
    state: State<'_, AppState>,
    plugin_id: String,
    native_id: String,
    preferred_lang: Option<String>,
) -> Result<MediaDetail, String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    let lang = preferred_lang.unwrap_or_else(|| "en".to_string());
    let (detail, _bg) = map_err(
        state
            .engine
            .details_with_enrichment(&pid, &native_id, &lang)
            .await,
    )?;
    Ok(detail)
}

#[tauri::command]
pub async fn get_streams(
    state: State<'_, AppState>,
    plugin_id: String,
    episode_id: String,
) -> Result<StreamResult, String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.streams(&pid, &episode_id).await)
}

#[tauri::command]
pub async fn set_metadata_override(
    state: State<'_, AppState>,
    plugin_id: String,
    native_id: String,
    canonical_id: Option<String>,
    is_disabled: bool,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    let media = map_err(MediaRef::new(pid, native_id))?;
    map_err(
        state
            .engine
            .set_metadata_override(&media, canonical_id.as_deref(), is_disabled)
            .await,
    )
}

#[tauri::command]
pub async fn list_library(
    state: State<'_, AppState>,
    status: Option<LibraryStatus>,
) -> Result<Vec<LibraryItem>, String> {
    map_err(state.engine.list_library(status).await)
}

#[tauri::command]
pub async fn upsert_library(state: State<'_, AppState>, item: LibraryItem) -> Result<(), String> {
    map_err(state.engine.upsert_library(&item).await)
}

#[tauri::command]
pub async fn delete_library_item(
    state: State<'_, AppState>,
    plugin_id: String,
    native_id: String,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    let media = map_err(MediaRef::new(pid, native_id))?;
    map_err(state.engine.delete_library_item(&media).await)
}

#[tauri::command]
pub async fn clear_runtime_caches(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.clear_runtime_caches().await;
    Ok(())
}

fn decode_subtitle_bytes(bytes: &[u8]) -> String {
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        return utf8.trim_start_matches('\u{feff}').to_owned();
    }
    // Windows-1254 (Turkish) fallback for legacy SRT/ASS files
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let ch = match b {
            0x80..=0x9F => match b {
                0x80 => '€',
                0x85 => '…',
                0x91 | 0x92 => '\'',
                0x93 | 0x94 => '"',
                0x96 | 0x97 => '-',
                _ => ' ',
            },
            0xD0 => 'Ğ',
            0xDD => 'İ',
            0xDE => 'Ş',
            0xF0 => 'ğ',
            0xFD => 'ı',
            0xFE => 'ş',
            _ => char::from(b),
        };
        out.push(ch);
    }
    out
}

#[tauri::command]
pub async fn fetch_subtitle_text(
    state: State<'_, AppState>,
    url: String,
    headers: Option<BTreeMap<String, String>>,
) -> Result<String, String> {
    let parsed = url::Url::parse(url.trim())
        .map_err(|err| format!("[VesselInvalidInputError]: invalid subtitle URL: {err}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("[VesselSecurityError]: unsupported subtitle URL scheme".into());
    }

    let proxy_base = state.engine.proxy().base_url();
    let is_local_proxy = url.starts_with(&proxy_base);
    if !is_local_proxy && let Some(host) = parsed.host_str() {
        if let Ok(ip) = host.parse::<std::net::IpAddr>()
            && vessel_plugin_rt::security::is_forbidden_ip(ip)
        {
            return Err(format!(
                "[VesselSecurityError]: blocked private subtitle host '{host}'"
            ));
        }
        if let Ok(addrs) =
            tokio::net::lookup_host((host, parsed.port_or_known_default().unwrap_or(443))).await
        {
            for addr in addrs {
                if vessel_plugin_rt::security::is_forbidden_ip(addr.ip()) {
                    return Err(format!(
                        "[VesselSecurityError]: blocked resolved private subtitle IP for '{host}'"
                    ));
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("Vessel-Desktop/1.0")
        .build()
        .map_err(|err| format!("[VesselNetworkError]: build subtitle client: {err}"))?;

    let mut req = client.get(parsed);
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(&v),
            ) {
                req = req.header(name, val);
            }
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|err| format!("[VesselNetworkError]: fetch subtitle failed: {err}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "[VesselNetworkError]: subtitle server returned HTTP {}",
            resp.status()
        ));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|err| format!("[VesselNetworkError]: read subtitle payload: {err}"))?;
    Ok(decode_subtitle_bytes(&bytes))
}

#[tauri::command]
pub async fn prepare_playback_url(
    state: State<'_, AppState>,
    candidate: StreamCandidate,
) -> Result<String, String> {
    map_err(state.engine.prepare_playback_url(&candidate))
}

#[tauri::command]
pub async fn play_candidate(
    state: State<'_, AppState>,
    payload: PlayCandidatePayload,
) -> Result<String, String> {
    let pid = map_err(PluginId::new(payload.plugin_id))?;
    let media = map_err(MediaRef::new(pid, payload.native_id))?;
    map_err(
        state
            .engine
            .play_candidate(PlayCandidateRequest {
                media,
                canonical_id: payload.canonical_id,
                season: payload.season,
                episode: payload.episode,
                absolute_episode: payload.absolute_episode,
                media_type: payload.media_type,
                candidate: payload.candidate,
                extra_subtitles: payload.extra_subtitles,
                start_position_ms: payload.start_position_ms,
                title: payload.title,
            })
            .await,
    )
}

#[tauri::command]
pub async fn record_progress(
    state: State<'_, AppState>,
    progress: PlaybackProgress,
) -> Result<(), String> {
    map_err(state.engine.record_progress(&progress).await)
}

#[tauri::command]
pub async fn get_watch_progress(
    state: State<'_, AppState>,
    plugin_id: String,
    native_id: String,
    season: u32,
    episode: u32,
) -> Result<Option<PlaybackProgress>, String> {
    let media = MediaRef {
        plugin_id: map_err(PluginId::new(plugin_id))?,
        native_id,
    };
    map_err(
        state
            .engine
            .get_watch_progress(&media, season, episode)
            .await,
    )
}

#[tauri::command]
pub async fn list_continue_watching(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ContinueWatchingEntry>, String> {
    map_err(
        state
            .engine
            .list_continue_watching(limit.unwrap_or(20))
            .await,
    )
}

#[tauri::command]
pub async fn delete_watch_progress(
    state: State<'_, AppState>,
    plugin_id: String,
    native_id: String,
) -> Result<(), String> {
    let pid = map_err(PluginId::new(plugin_id))?;
    map_err(state.engine.delete_watch_progress(&pid, &native_id).await)
}

#[tauri::command]
pub async fn player_play(state: State<'_, AppState>) -> Result<(), String> {
    map_err(state.engine.play().await)
}

#[tauri::command]
pub async fn player_pause(state: State<'_, AppState>) -> Result<(), String> {
    map_err(state.engine.pause().await)
}

#[tauri::command]
pub async fn player_seek(state: State<'_, AppState>, position_ms: u64) -> Result<(), String> {
    map_err(state.engine.seek(position_ms).await)
}

#[tauri::command]
pub async fn player_stop(state: State<'_, AppState>) -> Result<(), String> {
    map_err(state.engine.stop().await)
}

#[tauri::command]
pub async fn add_subtitle(state: State<'_, AppState>, track: SubtitleTrack) -> Result<(), String> {
    map_err(state.engine.add_subtitle(track).await)
}

#[tauri::command]
pub async fn set_subtitle_delay(state: State<'_, AppState>, delay_ms: i64) -> Result<(), String> {
    map_err(state.engine.set_subtitle_delay(delay_ms).await)
}

#[tauri::command]
pub async fn list_iptv_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<StoredIptvProfile>, String> {
    map_err(state.engine.list_iptv_profiles().await)
}

#[tauri::command]
pub async fn upsert_iptv_profile(
    state: State<'_, AppState>,
    payload: UpsertIptvProfilePayload,
) -> Result<StoredIptvProfile, String> {
    let profile = IptvProfile {
        id: payload.id,
        name: payload.name,
        kind: payload.kind,
        server_url: payload.server_url,
        username: payload.username,
        secret_ref: None,
        epg_url: payload.epg_url,
        user_agent: payload.user_agent,
        last_synced_at: None,
    };
    map_err(
        state
            .engine
            .upsert_iptv_profile(profile, payload.password.as_deref())
            .await,
    )
}

#[tauri::command]
pub async fn delete_iptv_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    map_err(state.engine.delete_iptv_profile(&profile_id).await)
}

#[tauri::command]
pub async fn sync_iptv_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<usize, String> {
    map_err(state.engine.sync_iptv_profile(&profile_id).await)
}

#[tauri::command]
pub async fn sync_iptv_epg(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<usize, String> {
    map_err(state.engine.sync_iptv_epg(&profile_id).await)
}

#[tauri::command]
pub async fn search_iptv_channels(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    query: Option<String>,
    group_title: Option<String>,
    favorites_only: Option<bool>,
    limit: Option<usize>,
) -> Result<Vec<StoredIptvChannel>, String> {
    map_err(
        state
            .engine
            .search_iptv_channels(
                profile_id.as_deref(),
                query.as_deref(),
                group_title.as_deref(),
                favorites_only.unwrap_or(false),
                limit.unwrap_or(200),
            )
            .await,
    )
}

#[tauri::command]
pub async fn list_iptv_groups(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Vec<String>, String> {
    map_err(state.engine.list_iptv_groups(&profile_id).await)
}

#[tauri::command]
pub async fn set_iptv_favorite(
    state: State<'_, AppState>,
    channel_id: i64,
    is_favorite: bool,
) -> Result<(), String> {
    map_err(
        state
            .engine
            .set_iptv_favorite(channel_id, is_favorite)
            .await,
    )
}

#[tauri::command]
pub async fn get_epg_now_next(
    state: State<'_, AppState>,
    profile_id: String,
    channel_tvg_id: String,
    now_ts: i64,
) -> Result<EpgNowNext, String> {
    map_err(
        state
            .engine
            .get_epg_now_next(&profile_id, &channel_tvg_id, now_ts)
            .await,
    )
}

#[tauri::command]
pub async fn play_iptv_channel(
    state: State<'_, AppState>,
    channel: StoredIptvChannel,
    profile_user_agent: Option<String>,
) -> Result<String, String> {
    map_err(
        state
            .engine
            .play_iptv_channel(&channel, profile_user_agent.as_deref())
            .await,
    )
}

#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<StoredDownload>, String> {
    map_err(state.engine.list_downloads().await)
}

#[tauri::command]
pub async fn enqueue_download(
    state: State<'_, AppState>,
    payload: EnqueueDownloadPayload,
) -> Result<StoredDownload, String> {
    map_err(
        state
            .engine
            .enqueue_download(DownloadRequest {
                id: None,
                plugin_id: payload.plugin_id,
                native_id: payload.native_id,
                title: payload.title,
                season: payload.season,
                episode: payload.episode,
                kind: payload.kind,
                source_url: payload.source_url,
                headers: payload.headers,
                subtitles: payload.subtitles,
            })
            .await,
    )
}

#[tauri::command]
pub async fn pause_download(state: State<'_, AppState>, download_id: String) -> Result<(), String> {
    map_err(state.engine.pause_download(&download_id).await)
}

#[tauri::command]
pub async fn resume_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<StoredDownload, String> {
    map_err(state.engine.resume_download(&download_id).await)
}

#[tauri::command]
pub async fn delete_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<(), String> {
    map_err(state.engine.delete_download(&download_id).await)
}

#[tauri::command]
pub async fn export_backup_json(
    state: State<'_, AppState>,
    include_secrets: bool,
) -> Result<String, String> {
    map_err(state.engine.export_backup_json(include_secrets).await)
}

#[tauri::command]
pub async fn import_backup_json(
    state: State<'_, AppState>,
    json: String,
    mode: BackupImportMode,
) -> Result<(), String> {
    map_err(state.engine.import_backup_json(&json, mode).await)
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("[VesselInvalidInputError]: only http(s) URLs can be opened externally".into());
    }

    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(trimmed).spawn();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/C", "start", "", trimmed])
        .spawn();
    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("xdg-open").arg(trimmed).spawn();

    status.map_err(|err| format!("[VesselSystemError]: failed to open external URL: {err}"))?;
    Ok(())
}
