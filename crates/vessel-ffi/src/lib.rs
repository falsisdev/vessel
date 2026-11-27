//! UniFFI Android/Mobile bridge exposing `VesselEngine` and streaming `EngineEvent` callbacks to Kotlin.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinSet;
use vessel_domain::{
    DebridPreferences, DebridProviderKind, EngineEvent, EventSink, ExternalIds, HomeSection,
    LibraryItem, LibraryStatus, LinkedMediaSource, MediaType, PlaybackProgress, PlaybackRequest,
    PlayerBackend, PluginId, PluginManifest, RepoIndex, ResolveDebridRequest, SecretStore,
    SkipSegment, StreamCandidate, SubtitleQuery, SubtitleTrack, ThemeColors, ThemeSpec,
    UiPreferences, VesselError, VesselPaths, VesselResult,
};
use vessel_engine::{
    DownloadRequest, IptvKind, IptvProfile, MetadataProviderFilter, PlayCandidateRequest,
    SIMKL_CLIENT_ID_SECRET, SkipQuery, TMDB_API_KEY_SECRET, VesselEngine,
};
use vessel_plugin_rt::{ResolvedSource, SourceTrustBadge};
use vessel_storage::{BackupImportMode, DownloadKind};

uniffi::setup_scaffolding!();

/// Initializes `rustls-platform-verifier` on Android with the Application context before any HTTP call.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_falsis_vessel_VesselApp_nativeInitTls<'local>(
    mut unowned_env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    context: jni::objects::JObject<'local>,
) {
    use jni::errors::ThrowRuntimeExAndDefault;
    unowned_env
        .with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<ThrowRuntimeExAndDefault>();
}

/// Unified FFI error preserving Vessel's `[Vessel<Variant>Error]: <message>` format.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("{reason}")]
    Vessel { reason: String },
}

impl From<VesselError> for FfiError {
    fn from(err: VesselError) -> Self {
        Self::Vessel {
            reason: err.to_string(),
        }
    }
}

fn ffi_err(msg: impl Into<String>) -> FfiError {
    FfiError::Vessel { reason: msg.into() }
}

fn to_json<T: Serialize>(value: &T) -> Result<String, FfiError> {
    serde_json::to_string(value)
        .map_err(|err| ffi_err(format!("[VesselSerializationError]: {err}")))
}

fn from_json<T: for<'de> Deserialize<'de>>(raw: &str, label: &str) -> Result<T, FfiError> {
    serde_json::from_str(raw).map_err(|err| {
        ffi_err(format!(
            "[VesselInvalidInputError]: invalid {label} JSON: {err}"
        ))
    })
}

/// Callback implemented in Kotlin to stream real-time `EngineEvent` JSON payloads from Rust.
#[uniffi::export(callback_interface)]
pub trait EngineEventListener: Send + Sync {
    fn on_event_json(&self, event_json: String);
}

struct FfiEventSink {
    listener: Arc<Box<dyn EngineEventListener>>,
}

impl EventSink for FfiEventSink {
    fn emit(&self, event: EngineEvent) {
        if let Ok(json) = serde_json::to_string(&event) {
            self.listener.on_event_json(json);
        }
    }
}

/// Lightweight Android no-op `PlayerBackend`: Media3 ExoPlayer plays directly via proxy URLs.
struct MobilePlayerNoopBackend;

#[async_trait]
impl PlayerBackend for MobilePlayerNoopBackend {
    async fn load(&self, _request: PlaybackRequest) -> VesselResult<()> {
        Ok(())
    }

    async fn add_subtitle(&self, _track: SubtitleTrack) -> VesselResult<()> {
        Ok(())
    }

    async fn set_subtitle_delay(&self, _delay_ms: i64) -> VesselResult<()> {
        Ok(())
    }

    async fn play(&self) -> VesselResult<()> {
        Ok(())
    }

    async fn pause(&self) -> VesselResult<()> {
        Ok(())
    }

    async fn seek(&self, _position_ms: u64) -> VesselResult<()> {
        Ok(())
    }

    async fn stop(&self) -> VesselResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineInfoDto {
    root_dir: String,
    downloads_dir: String,
    plugins_dir: String,
    themes_dir: String,
    locales_dir: String,
    backups_dir: String,
    proxy_base_url: String,
    always_proxy: bool,
    metadata_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginListEntryDto {
    manifest: PluginManifest,
    repo_id: Option<String>,
    source_type: String,
    source_url: Option<String>,
    local_path: String,
    sha256: Option<String>,
    enabled: bool,
    allow_dynamic_domains: bool,
    allowed_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePluginResultDto {
    plugin_id: String,
    badge: String,
    version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResolvedSourceDto {
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
struct HomeShelfResultDto {
    plugin_id: String,
    plugin_name: String,
    sections: Vec<HomeSection>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallResultDto {
    plugin_id: String,
    badge: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayCandidatePayload {
    plugin_id: String,
    native_id: String,
    canonical_id: Option<String>,
    season: u32,
    episode: u32,
    absolute_episode: Option<u32>,
    media_type: vessel_domain::MediaType,
    candidate: StreamCandidate,
    #[serde(default)]
    extra_subtitles: Vec<SubtitleTrack>,
    start_position_ms: Option<u64>,
    title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnqueueDownloadPayload {
    plugin_id: String,
    native_id: String,
    title: String,
    season: Option<u32>,
    episode: Option<u32>,
    stream_label: Option<String>,
    kind: DownloadKind,
    source_url: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    subtitles: Vec<SubtitleTrack>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertIptvProfilePayload {
    id: String,
    name: String,
    kind: IptvKind,
    server_url: String,
    username: Option<String>,
    password: Option<String>,
    epg_url: Option<String>,
    user_agent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetSkipSegmentsPayload {
    plugin_id: Option<String>,
    native_id: Option<String>,
    media_type: vessel_domain::MediaType,
    #[serde(default)]
    is_anime: bool,
    #[serde(default)]
    external_ids: ExternalIds,
    #[serde(default)]
    season: u32,
    #[serde(default)]
    episode: u32,
    absolute_episode: Option<u32>,
    duration_ms: Option<u64>,
    #[serde(default)]
    plugin_segments: Vec<SkipSegment>,
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

/// Core runtime object owned by the Android/Mobile host app over UniFFI.
#[derive(uniffi::Object)]
pub struct VesselMobileCore {
    runtime: Arc<Runtime>,
    engine: Arc<VesselEngine>,
    paths: VesselPaths,
}

#[uniffi::export]
impl VesselMobileCore {
    /// Initializes SQLite, proxy, downloader, plugins, and background tasks inside a dedicated Tokio pool.
    #[uniffi::constructor]
    pub fn open(
        root_dir: String,
        data_dir: String,
        listener: Option<Box<dyn EngineEventListener>>,
    ) -> Result<Arc<Self>, FfiError> {
        let paths = VesselPaths::with_data_dir(root_dir, data_dir)?;
        let runtime = Arc::new(
            Builder::new_multi_thread()
                .enable_all()
                .thread_name("vessel-mobile-tokio")
                .build()
                .map_err(|err| {
                    ffi_err(format!("[VesselRuntimeError]: build tokio runtime: {err}"))
                })?,
        );

        let event_sink: Arc<dyn EventSink> = match listener {
            Some(cb) => Arc::new(FfiEventSink {
                listener: Arc::new(cb),
            }),
            None => Arc::new(NoopEventSink),
        };
        let player: Arc<dyn PlayerBackend> = Arc::new(MobilePlayerNoopBackend);

        let engine = runtime.block_on(async {
            let eng = VesselEngine::open(&paths, player, event_sink, false).await?;
            if let Ok(Some(always_proxy)) =
                eng.storage().get_json_setting::<bool>("always_proxy").await
            {
                eng.set_always_proxy(always_proxy);
            }
            if let Ok(Some(meta_enabled)) = eng
                .storage()
                .get_json_setting::<bool>("metadata_enabled")
                .await
            {
                eng.set_metadata_enabled(meta_enabled);
            }
            if let Ok(Some(custom_dl_dir)) = eng
                .storage()
                .get_json_setting::<String>("custom_download_dir")
                .await
            {
                let trimmed = custom_dl_dir.trim();
                if !trimmed.is_empty() {
                    let _ = eng.set_downloads_dir(trimmed);
                }
            }
            let _ = eng.bootstrap_plugins().await;
            Ok::<_, VesselError>(Arc::new(eng))
        })?;

        let reaper_engine = Arc::clone(&engine);
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(45));
            loop {
                interval.tick().await;
                reaper_engine.evict_idle_plugins().await;
            }
        });

        Ok(Arc::new(Self {
            runtime,
            engine,
            paths,
        }))
    }

    pub fn get_engine_info_json(&self) -> Result<String, FfiError> {
        to_json(&EngineInfoDto {
            root_dir: self.paths.root().to_string_lossy().into_owned(),
            downloads_dir: self.engine.downloads_dir().to_string_lossy().into_owned(),
            plugins_dir: self.paths.plugins_dir().to_string_lossy().into_owned(),
            themes_dir: self.paths.themes_dir().to_string_lossy().into_owned(),
            locales_dir: self.paths.locales_dir().to_string_lossy().into_owned(),
            backups_dir: self.paths.backups_dir().to_string_lossy().into_owned(),
            proxy_base_url: self.engine.proxy().base_url(),
            always_proxy: self.engine.always_proxy(),
            metadata_enabled: self.engine.metadata_enabled(),
        })
    }

    pub fn proxy_base_url(&self) -> String {
        self.engine.proxy().base_url()
    }

    pub fn set_always_proxy(&self, enabled: bool) -> Result<(), FfiError> {
        self.engine.set_always_proxy(enabled);
        self.runtime.block_on(async {
            self.engine
                .storage()
                .set_json_setting("always_proxy", &enabled)
                .await
        })?;
        Ok(())
    }

    pub fn set_metadata_enabled(&self, enabled: bool) -> Result<(), FfiError> {
        self.engine.set_metadata_enabled(enabled);
        self.runtime.block_on(async {
            self.engine
                .storage()
                .set_json_setting("metadata_enabled", &enabled)
                .await
        })?;
        Ok(())
    }

    pub fn set_tmdb_api_key(&self, api_key: Option<String>) -> Result<(), FfiError> {
        let trimmed = api_key.unwrap_or_default().trim().to_owned();
        self.runtime.block_on(async {
            if trimmed.is_empty() {
                self.engine.secrets().delete(TMDB_API_KEY_SECRET).await
            } else {
                self.engine
                    .secrets()
                    .set(TMDB_API_KEY_SECRET, &trimmed)
                    .await
            }
        })?;
        Ok(())
    }

    pub fn set_simkl_client_id(&self, client_id: Option<String>) -> Result<(), FfiError> {
        let trimmed = client_id.unwrap_or_default().trim().to_owned();
        self.runtime.block_on(async {
            if trimmed.is_empty() {
                self.engine.secrets().delete(SIMKL_CLIENT_ID_SECRET).await
            } else {
                self.engine
                    .secrets()
                    .set(SIMKL_CLIENT_ID_SECRET, &trimmed)
                    .await
            }
        })?;
        Ok(())
    }

    pub fn set_download_directory(&self, path: Option<String>) -> Result<String, FfiError> {
        let cleaned = path.unwrap_or_default().trim().to_owned();
        let target = if cleaned.is_empty() {
            self.paths.downloads_dir()
        } else {
            std::path::PathBuf::from(&cleaned)
        };
        fs::create_dir_all(&target)
            .map_err(|err| ffi_err(format!("[VesselStorageError]: create download dir: {err}")))?;
        self.engine.set_downloads_dir(&target)?;
        self.runtime.block_on(async {
            self.engine
                .storage()
                .set_json_setting("custom_download_dir", &cleaned)
                .await
        })?;
        Ok(self.engine.downloads_dir().to_string_lossy().into_owned())
    }

    pub fn clear_runtime_caches(&self) {
        self.runtime.block_on(self.engine.clear_runtime_caches());
    }

    pub fn get_ui_preferences_json(&self) -> Result<String, FfiError> {
        let prefs = self.runtime.block_on(self.engine.get_ui_preferences())?;
        to_json(&prefs)
    }

    pub fn set_ui_preferences_json(&self, prefs_json: String) -> Result<(), FfiError> {
        let prefs: UiPreferences = from_json(&prefs_json, "UiPreferences")?;
        self.runtime
            .block_on(self.engine.set_ui_preferences(&prefs))?;
        Ok(())
    }

    pub fn list_themes_json(&self) -> Result<String, FfiError> {
        let mut themes = builtin_themes();
        let themes_dir = self.paths.themes_dir();
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
        to_json(&themes)
    }

    pub fn import_theme_json(&self, theme_json: String) -> Result<String, FfiError> {
        let spec: ThemeSpec = from_json(&theme_json, "ThemeSpec")?;
        spec.validate()?;
        let themes_dir = self.paths.themes_dir();
        fs::create_dir_all(&themes_dir)
            .map_err(|err| ffi_err(format!("[VesselStorageError]: create themes dir: {err}")))?;
        let safe_id: String = spec
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let target = themes_dir.join(format!("{safe_id}.theme.vessel"));
        fs::write(&target, theme_json.as_bytes())
            .map_err(|err| ffi_err(format!("[VesselStorageError]: write theme file: {err}")))?;
        to_json(&spec)
    }

    pub fn export_backup_json(&self, include_secrets: bool) -> Result<String, FfiError> {
        Ok(self
            .runtime
            .block_on(self.engine.export_backup_json(include_secrets))?)
    }

    pub fn import_backup_json(&self, backup_json: String, mode: String) -> Result<(), FfiError> {
        let import_mode = match mode.to_ascii_lowercase().as_str() {
            "overwrite" | "reset_and_replace" | "reset_and_import" | "reset" => {
                BackupImportMode::ResetAndImport
            }
            _ => BackupImportMode::SmartMerge,
        };
        self.runtime
            .block_on(self.engine.import_backup_json(&backup_json, import_mode))?;
        Ok(())
    }

    pub fn list_plugins_json(&self) -> Result<String, FfiError> {
        let out = self.runtime.block_on(async {
            let stored = self.engine.list_plugins().await?;
            let mut entries = Vec::with_capacity(stored.len());
            for item in stored {
                let allowed_domains = self.engine.list_allowed_domains(&item.manifest.id).await?;
                entries.push(PluginListEntryDto {
                    manifest: item.manifest,
                    repo_id: item.repo_id,
                    source_type: item.source_type,
                    source_url: item.source_url,
                    local_path: item.local_path,
                    sha256: item.sha256,
                    enabled: item.enabled,
                    allow_dynamic_domains: item.allow_dynamic_domains,
                    allowed_domains,
                });
            }
            Ok::<_, VesselError>(entries)
        })?;
        to_json(&out)
    }

    pub fn resolve_source_json(&self, raw_input: String) -> Result<String, FfiError> {
        let resolved = self
            .runtime
            .block_on(self.engine.resolve_source(&raw_input))?;
        let dto = match resolved {
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
        };
        to_json(&dto)
    }

    pub fn install_source_json(
        &self,
        raw_input: String,
        repo_id: Option<String>,
    ) -> Result<String, FfiError> {
        let plugins_dir = self.paths.plugins_dir();
        let (id, badge) =
            self.runtime
                .block_on(self.engine.install_package_persisted_with_repo(
                    &plugins_dir,
                    &raw_input,
                    repo_id,
                ))?;
        to_json(&InstallResultDto {
            plugin_id: id.to_string(),
            badge: badge_label(badge).into(),
        })
    }

    pub fn set_plugin_enabled(&self, plugin_id: String, enabled: bool) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        self.runtime
            .block_on(self.engine.set_plugin_enabled(&pid, enabled))?;
        Ok(())
    }

    pub fn uninstall_plugin(&self, plugin_id: String) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        self.runtime.block_on(self.engine.uninstall_plugin(&pid))?;
        Ok(())
    }

    pub fn check_plugin_updates_json(&self) -> Result<String, FfiError> {
        let out = self.runtime.block_on(self.engine.check_plugin_updates())?;
        to_json(&out)
    }

    pub fn update_plugin_json(&self, plugin_id: String) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let plugins_dir = self.paths.plugins_dir();
        let (id, badge, version) = self
            .runtime
            .block_on(self.engine.update_plugin(&plugins_dir, &pid))?;
        to_json(&UpdatePluginResultDto {
            plugin_id: id.to_string(),
            badge: badge_label(badge).into(),
            version,
        })
    }

    pub fn update_all_plugins_json(&self) -> Result<String, FfiError> {
        let plugins_dir = self.paths.plugins_dir();
        let out = self
            .runtime
            .block_on(self.engine.update_all_plugins(&plugins_dir))?;
        to_json(&out)
    }

    pub fn grant_plugin_domain(
        &self,
        plugin_id: String,
        domain: String,
        permanent: bool,
    ) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        self.runtime
            .block_on(self.engine.grant_plugin_domain(&pid, &domain, permanent))?;
        Ok(())
    }

    pub fn get_plugin_settings_json(&self, plugin_id: String) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let map = self
            .runtime
            .block_on(self.engine.get_plugin_settings(&pid))?;
        to_json(&map)
    }

    pub fn set_plugin_settings_json(
        &self,
        plugin_id: String,
        values_json: String,
    ) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let values: BTreeMap<String, serde_json::Value> =
            from_json(&values_json, "plugin settings")?;
        self.runtime
            .block_on(self.engine.set_plugin_settings(&pid, values))?;
        Ok(())
    }

    pub fn get_home_feeds_json(&self, plugin_ids: Option<Vec<String>>) -> Result<String, FfiError> {
        let results = self.runtime.block_on(async {
            let plugins = self.engine.list_plugins().await?;
            let filter_set: Option<HashSet<String>> =
                plugin_ids.map(|ids| ids.into_iter().collect());
            let sem = Arc::new(tokio::sync::Semaphore::new(8));
            let mut set = JoinSet::new();

            for stored in plugins.into_iter().filter(|p| {
                p.enabled
                    && filter_set
                        .as_ref()
                        .is_none_or(|ids| ids.contains(p.manifest.id.as_str()))
            }) {
                let engine = Arc::clone(&self.engine);
                let sem = Arc::clone(&sem);
                set.spawn(async move {
                    let _permit = sem.acquire_owned().await.ok();
                    let pid = stored.manifest.id.clone();
                    let name = stored.manifest.name.clone();
                    match engine.home_feed(&pid, None).await {
                        Ok(feed) => HomeShelfResultDto {
                            plugin_id: pid.to_string(),
                            plugin_name: name,
                            sections: feed.sections,
                            error: None,
                        },
                        Err(err) => HomeShelfResultDto {
                            plugin_id: pid.to_string(),
                            plugin_name: name,
                            sections: Vec::new(),
                            error: Some(err.to_string()),
                        },
                    }
                });
            }

            let mut collected = Vec::new();
            while let Some(res) = set.join_next().await {
                if let Ok(item) = res {
                    collected.push(item);
                }
            }
            collected.sort_by(|a, b| a.plugin_name.cmp(&b.plugin_name));
            Ok::<_, VesselError>(collected)
        })?;
        to_json(&results)
    }

    pub fn search_streams(
        &self,
        query: String,
        filters_json: Option<String>,
        page: Option<u32>,
        plugin_ids: Option<Vec<String>>,
    ) -> Result<String, FfiError> {
        let filters: BTreeMap<String, String> = match filters_json {
            Some(raw) if !raw.trim().is_empty() => from_json(&raw, "search filters")?,
            _ => BTreeMap::new(),
        };
        let parsed_plugins = match plugin_ids {
            Some(ids) if !ids.is_empty() => Some(
                ids.into_iter()
                    .map(PluginId::new)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            _ => None,
        };
        let handle = self.runtime.block_on(self.engine.search_selected(
            &query,
            filters,
            page.unwrap_or(1),
            parsed_plugins,
        ));
        Ok(handle.search_id)
    }

    pub fn cancel_search(&self) {
        self.runtime.block_on(self.engine.cancel_active_search());
    }

    pub fn get_details_json(
        &self,
        plugin_id: String,
        native_id: String,
        preferred_lang: Option<String>,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let lang = preferred_lang.unwrap_or_else(|| "en".to_owned());
        let (detail, _bg) = self
            .runtime
            .block_on(self.engine.details_with_enrichment(&pid, &native_id, &lang))?;
        to_json(&detail)
    }

    pub fn get_streams_json(
        &self,
        plugin_id: String,
        episode_id: String,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let streams = self
            .runtime
            .block_on(self.engine.streams(&pid, &episode_id))?;
        to_json(&streams)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_streams_with_linked_json(
        &self,
        plugin_id: String,
        native_id: String,
        episode_id: String,
        season: u32,
        episode: u32,
        absolute_episode: Option<u32>,
        is_movie: bool,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, native_id)?;
        let res = self.runtime.block_on(self.engine.streams_with_linked(
            &media,
            &episode_id,
            season,
            episode,
            absolute_episode,
            is_movie,
        ))?;
        to_json(&res)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn get_streams_with_external_ids_json(
        &self,
        plugin_id: String,
        native_id: String,
        episode_id: String,
        season: u32,
        episode: u32,
        absolute_episode: Option<u32>,
        is_movie: bool,
        external_ids_json: Option<String>,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, native_id)?;
        let ext_ids: Option<ExternalIds> = match external_ids_json {
            Some(ref s) if !s.trim().is_empty() && s.trim() != "null" => {
                Some(from_json(s, "ExternalIds")?)
            }
            _ => None,
        };
        let res = self
            .runtime
            .block_on(self.engine.streams_with_external_ids(
                &media,
                &episode_id,
                season,
                episode,
                absolute_episode,
                is_movie,
                ext_ids.as_ref(),
            ))?;
        to_json(&res)
    }

    pub fn get_virtual_metadata_detail_json(
        &self,
        canonical_id: String,
        title_hint: Option<String>,
        media_type_hint: Option<String>,
        preferred_lang: Option<String>,
    ) -> Result<String, FfiError> {
        let mtype = match media_type_hint.as_deref() {
            Some("series") => Some(MediaType::Series),
            Some("live") => Some(MediaType::Live),
            Some("movie") => Some(MediaType::Movie),
            _ => None,
        };
        let lang = preferred_lang.unwrap_or_else(|| "en".into());
        let detail = self.runtime.block_on(self.engine.get_virtual_metadata_detail(
            &canonical_id,
            title_hint.as_deref(),
            mtype,
            &lang,
        ))?;
        to_json(&detail)
    }

    pub fn get_debrid_overview_json(&self) -> Result<String, FfiError> {
        let prefs = self
            .runtime
            .block_on(self.engine.get_debrid_preferences())?;
        let accounts = self
            .runtime
            .block_on(self.engine.get_debrid_accounts_status())?;
        let disabled_stream_addons = self
            .runtime
            .block_on(self.engine.list_disabled_stream_addons())?;
        to_json(&serde_json::json!({
            "preferences": prefs,
            "accounts": accounts,
            "disabledStreamAddons": disabled_stream_addons,
        }))
    }

    pub fn set_debrid_preferences_json(&self, prefs_json: String) -> Result<String, FfiError> {
        let prefs: DebridPreferences = from_json(&prefs_json, "DebridPreferences")?;
        let saved = self
            .runtime
            .block_on(self.engine.set_debrid_preferences(prefs))?;
        to_json(&saved)
    }

    pub fn save_debrid_api_key(&self, provider: String, api_key: String) -> Result<(), FfiError> {
        let kind = DebridProviderKind::parse_id(&provider).ok_or_else(|| {
            VesselError::InvalidInput(format!("unknown Debrid provider '{provider}'"))
        })?;
        self.runtime
            .block_on(self.engine.save_debrid_api_key(kind, &api_key))?;
        Ok(())
    }

    pub fn remove_debrid_credentials(&self, provider: String) -> Result<(), FfiError> {
        let kind = DebridProviderKind::parse_id(&provider).ok_or_else(|| {
            VesselError::InvalidInput(format!("unknown Debrid provider '{provider}'"))
        })?;
        self.runtime
            .block_on(self.engine.remove_debrid_credentials(kind))?;
        Ok(())
    }

    pub fn start_real_debrid_device_auth_json(&self) -> Result<String, FfiError> {
        let start = self
            .runtime
            .block_on(self.engine.start_real_debrid_device_auth())?;
        to_json(&start)
    }

    pub fn poll_real_debrid_device_auth_json(
        &self,
        device_code: String,
    ) -> Result<String, FfiError> {
        let poll = self
            .runtime
            .block_on(self.engine.poll_real_debrid_device_auth(&device_code))?;
        to_json(&poll)
    }

    pub fn probe_torrent_cache_json(&self, candidates_json: String) -> Result<String, FfiError> {
        let candidates: Vec<StreamCandidate> = from_json(&candidates_json, "Vec<StreamCandidate>")?;
        let probed = self
            .runtime
            .block_on(self.engine.probe_torrent_cache(candidates))?;
        to_json(&probed)
    }

    pub fn resolve_torrent_candidate_json(&self, request_json: String) -> Result<String, FfiError> {
        let req: ResolveDebridRequest = from_json(&request_json, "ResolveDebridRequest")?;
        let res = self
            .runtime
            .block_on(self.engine.resolve_torrent_candidate(req))?;
        to_json(&res)
    }

    pub fn list_debrid_cloud_json(&self) -> Result<String, FfiError> {
        let items = self.runtime.block_on(self.engine.list_debrid_cloud())?;
        to_json(&items)
    }

    pub fn delete_debrid_cloud(&self, target: String, torrent_id: String) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.delete_debrid_cloud(&target, &torrent_id))?;
        Ok(())
    }

    pub fn unrestrict_debrid_cloud_link_json(
        &self,
        target: String,
        link: String,
    ) -> Result<String, FfiError> {
        let resolved = self
            .runtime
            .block_on(self.engine.unrestrict_debrid_cloud_link(&target, &link))?;
        to_json(&resolved)
    }

    pub fn set_stream_addon_auto_enabled(
        &self,
        plugin_id: String,
        enabled: bool,
    ) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        self.runtime
            .block_on(self.engine.set_stream_addon_auto_enabled(&pid, enabled))?;
        Ok(())
    }

    pub fn get_playback_stats_json(&self) -> Result<String, FfiError> {
        let stats = self.runtime.block_on(self.engine.get_playback_stats());
        to_json(&stats)
    }

    pub fn search_plugin_sync_json(
        &self,
        plugin_id: String,
        query: String,
        page: Option<u32>,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let res = self.runtime.block_on(self.engine.search_plugin_sync(
            &pid,
            &query,
            page.unwrap_or(1),
        ))?;
        to_json(&res)
    }

    pub fn search_metadata_json(
        &self,
        query: String,
        provider: Option<String>,
        preferred_lang: Option<String>,
    ) -> Result<String, FfiError> {
        let filter = match provider.as_deref().map(str::trim).unwrap_or("all") {
            "tmdb" | "tmdb_tv" | "tmdb_movie" => MetadataProviderFilter::Tmdb,
            "imdb" => MetadataProviderFilter::Imdb,
            "simkl" => MetadataProviderFilter::Simkl,
            "anilist" => MetadataProviderFilter::Anilist,
            "mal" => MetadataProviderFilter::Mal,
            "kitsu" => MetadataProviderFilter::Kitsu,
            _ => MetadataProviderFilter::All,
        };
        let lang = preferred_lang.unwrap_or_else(|| "en".into());
        let res = self
            .runtime
            .block_on(self.engine.search_metadata(&query, filter, &lang))?;
        to_json(&res)
    }

    pub fn get_linked_sources_json(
        &self,
        plugin_id: String,
        native_id: String,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, native_id)?;
        let list = self
            .runtime
            .block_on(self.engine.get_linked_sources(&media))?;
        to_json(&list)
    }

    pub fn set_linked_sources_json(
        &self,
        plugin_id: String,
        native_id: String,
        sources_json: String,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, native_id)?;
        let sources: Vec<LinkedMediaSource> = from_json(&sources_json, "Vec<LinkedMediaSource>")?;
        let saved = self
            .runtime
            .block_on(self.engine.set_linked_sources(&media, sources))?;
        to_json(&saved)
    }

    pub fn set_metadata_override(
        &self,
        plugin_id: String,
        native_id: String,
        canonical_id: Option<String>,
        is_disabled: bool,
    ) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, native_id)?;
        self.runtime.block_on(self.engine.set_metadata_override(
            &media,
            canonical_id.as_deref(),
            is_disabled,
        ))?;
        Ok(())
    }

    pub fn get_skip_segments_json(&self, payload_json: String) -> Result<String, FfiError> {
        let payload: GetSkipSegmentsPayload = from_json(&payload_json, "GetSkipSegmentsPayload")?;
        let media = match (payload.plugin_id, payload.native_id) {
            (Some(p), Some(n)) if !p.trim().is_empty() && !n.trim().is_empty() => {
                Some(vessel_domain::MediaRef::new(PluginId::new(p)?, n)?)
            }
            _ => None,
        };
        let segs = self.runtime.block_on(self.engine.get_skip_segments(
            media.as_ref(),
            SkipQuery {
                media_type: payload.media_type,
                is_anime: payload.is_anime,
                external_ids: payload.external_ids,
                season: payload.season,
                episode: payload.episode,
                absolute_episode: payload.absolute_episode,
                duration_ms: payload.duration_ms,
                plugin_segments: payload.plugin_segments,
            },
        ));
        to_json(&segs)
    }

    pub fn prepare_playback_url(&self, candidate_json: String) -> Result<String, FfiError> {
        let candidate: StreamCandidate = from_json(&candidate_json, "StreamCandidate")?;
        Ok(self.engine.prepare_playback_url(&candidate)?)
    }

    pub fn list_subtitle_plugins_json(&self) -> Result<String, FfiError> {
        let list = self.runtime.block_on(self.engine.list_subtitle_plugins());
        to_json(&list)
    }

    pub fn search_subtitles_json(
        &self,
        query_json: String,
        target_plugins: Option<Vec<String>>,
    ) -> Result<String, FfiError> {
        let query: SubtitleQuery = from_json(&query_json, "SubtitleQuery")?;
        let parsed_targets = match target_plugins {
            Some(list) => Some(
                list.into_iter()
                    .map(PluginId::new)
                    .collect::<VesselResult<Vec<_>>>()?,
            ),
            None => None,
        };
        let outcome = self
            .runtime
            .block_on(self.engine.search_subtitles(query, parsed_targets))?;
        to_json(&outcome)
    }

    pub fn resolve_subtitle_json(
        &self,
        plugin_id: String,
        subtitle_id: String,
        lang: String,
        label: String,
    ) -> Result<String, FfiError> {
        let pid = PluginId::new(plugin_id)?;
        let track = self.runtime.block_on(self.engine.resolve_subtitle(
            &pid,
            &subtitle_id,
            &lang,
            &label,
        ))?;
        to_json(&track)
    }

    pub fn play_candidate(&self, payload_json: String) -> Result<String, FfiError> {
        let payload: PlayCandidatePayload = from_json(&payload_json, "PlayCandidatePayload")?;
        let pid = PluginId::new(payload.plugin_id)?;
        let media = vessel_domain::MediaRef::new(pid, payload.native_id)?;
        Ok(self
            .runtime
            .block_on(self.engine.play_candidate(PlayCandidateRequest {
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
            }))?)
    }

    pub fn record_progress_json(&self, progress_json: String) -> Result<(), FfiError> {
        let progress: PlaybackProgress = from_json(&progress_json, "PlaybackProgress")?;
        self.runtime
            .block_on(self.engine.record_progress(&progress))?;
        Ok(())
    }

    pub fn get_watch_progress_json(
        &self,
        plugin_id: String,
        native_id: String,
        season: u32,
        episode: u32,
    ) -> Result<String, FfiError> {
        let media = vessel_domain::MediaRef::new(PluginId::new(plugin_id)?, native_id)?;
        let prog = self
            .runtime
            .block_on(self.engine.get_watch_progress(&media, season, episode))?;
        to_json(&prog)
    }

    pub fn list_continue_watching_json(&self, limit: Option<u32>) -> Result<String, FfiError> {
        let rows = self.runtime.block_on(
            self.engine
                .list_continue_watching(limit.unwrap_or(20) as usize),
        )?;
        to_json(&rows)
    }

    pub fn list_media_watch_progress_json(
        &self,
        plugin_id: String,
        native_id: String,
    ) -> Result<String, FfiError> {
        let media = vessel_domain::MediaRef::new(PluginId::new(plugin_id)?, native_id)?;
        let rows = self
            .runtime
            .block_on(self.engine.list_media_watch_progress(&media))?;
        to_json(&rows)
    }

    pub fn list_all_watch_progress_json(&self, limit: Option<u32>) -> Result<String, FfiError> {
        let rows = self.runtime.block_on(
            self.engine
                .list_all_watch_progress(limit.unwrap_or(200) as usize),
        )?;
        to_json(&rows)
    }

    pub fn delete_watch_progress(
        &self,
        plugin_id: String,
        native_id: String,
    ) -> Result<(), FfiError> {
        let pid = PluginId::new(plugin_id)?;
        self.runtime
            .block_on(self.engine.delete_watch_progress(&pid, &native_id))?;
        Ok(())
    }

    pub fn list_library_json(&self, status: Option<String>) -> Result<String, FfiError> {
        let parsed_status = match status.as_deref() {
            Some("watching") => Some(LibraryStatus::Watching),
            Some("plan_to_watch") => Some(LibraryStatus::PlanToWatch),
            Some("completed") => Some(LibraryStatus::Completed),
            Some("on_hold") => Some(LibraryStatus::OnHold),
            Some("dropped") => Some(LibraryStatus::Dropped),
            _ => None,
        };
        let items = self
            .runtime
            .block_on(self.engine.list_library(parsed_status))?;
        to_json(&items)
    }

    pub fn upsert_library_json(&self, item_json: String) -> Result<(), FfiError> {
        let item: LibraryItem = from_json(&item_json, "LibraryItem")?;
        self.runtime.block_on(self.engine.upsert_library(&item))?;
        Ok(())
    }

    pub fn delete_library_item(
        &self,
        plugin_id: String,
        native_id: String,
    ) -> Result<(), FfiError> {
        let media = vessel_domain::MediaRef::new(PluginId::new(plugin_id)?, native_id)?;
        self.runtime
            .block_on(self.engine.delete_library_item(&media))?;
        Ok(())
    }

    pub fn list_iptv_profiles_json(&self) -> Result<String, FfiError> {
        let list = self.runtime.block_on(self.engine.list_iptv_profiles())?;
        to_json(&list)
    }

    pub fn upsert_iptv_profile_json(&self, payload_json: String) -> Result<String, FfiError> {
        let payload: UpsertIptvProfilePayload =
            from_json(&payload_json, "UpsertIptvProfilePayload")?;
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
        let stored = self.runtime.block_on(
            self.engine
                .upsert_iptv_profile(profile, payload.password.as_deref()),
        )?;
        to_json(&stored)
    }

    pub fn delete_iptv_profile(&self, profile_id: String) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.delete_iptv_profile(&profile_id))?;
        Ok(())
    }

    pub fn sync_iptv_profile(&self, profile_id: String) -> Result<u32, FfiError> {
        let n = self
            .runtime
            .block_on(self.engine.sync_iptv_profile(&profile_id))?;
        Ok(n as u32)
    }

    pub fn sync_iptv_epg(&self, profile_id: String) -> Result<u32, FfiError> {
        let n = self
            .runtime
            .block_on(self.engine.sync_iptv_epg(&profile_id))?;
        Ok(n as u32)
    }

    pub fn search_iptv_channels_json(
        &self,
        profile_id: Option<String>,
        query: Option<String>,
        group_title: Option<String>,
        favorites_only: bool,
        limit: Option<u32>,
    ) -> Result<String, FfiError> {
        let channels = self.runtime.block_on(self.engine.search_iptv_channels(
            profile_id.as_deref(),
            query.as_deref(),
            group_title.as_deref(),
            favorites_only,
            limit.unwrap_or(200) as usize,
        ))?;
        to_json(&channels)
    }

    pub fn list_iptv_groups(&self, profile_id: String) -> Result<Vec<String>, FfiError> {
        Ok(self
            .runtime
            .block_on(self.engine.list_iptv_groups(&profile_id))?)
    }

    pub fn set_iptv_favorite(&self, channel_id: i64, is_favorite: bool) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.set_iptv_favorite(channel_id, is_favorite))?;
        Ok(())
    }

    pub fn get_epg_now_next_json(
        &self,
        profile_id: String,
        tvg_id: String,
        now_ts: Option<i64>,
    ) -> Result<String, FfiError> {
        let ts = now_ts.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
        let res = self
            .runtime
            .block_on(self.engine.get_epg_now_next(&profile_id, &tvg_id, ts))?;
        to_json(&res)
    }

    pub fn play_iptv_channel(
        &self,
        channel_json: String,
        profile_user_agent: Option<String>,
    ) -> Result<String, FfiError> {
        let channel: vessel_storage::StoredIptvChannel =
            from_json(&channel_json, "StoredIptvChannel")?;
        Ok(self.runtime.block_on(
            self.engine
                .play_iptv_channel(&channel, profile_user_agent.as_deref()),
        )?)
    }

    pub fn list_downloads_json(&self) -> Result<String, FfiError> {
        let list = self.runtime.block_on(self.engine.list_downloads())?;
        to_json(&list)
    }

    pub fn enqueue_download_json(&self, payload_json: String) -> Result<String, FfiError> {
        let payload: EnqueueDownloadPayload = from_json(&payload_json, "EnqueueDownloadPayload")?;
        let pid = PluginId::new(payload.plugin_id)?;
        let dl = self
            .runtime
            .block_on(self.engine.enqueue_download(DownloadRequest {
                id: None,
                plugin_id: pid.to_string(),
                native_id: payload.native_id,
                title: payload.title,
                season: payload.season,
                episode: payload.episode,
                stream_label: payload.stream_label,
                kind: payload.kind,
                source_url: payload.source_url,
                headers: payload.headers,
                subtitles: payload.subtitles,
            }))?;
        to_json(&dl)
    }

    pub fn pause_download(&self, download_id: String) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.pause_download(&download_id))?;
        Ok(())
    }

    pub fn resume_download_json(&self, download_id: String) -> Result<String, FfiError> {
        let dl = self
            .runtime
            .block_on(self.engine.resume_download(&download_id))?;
        to_json(&dl)
    }

    pub fn delete_download(&self, download_id: String) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.delete_download(&download_id))?;
        Ok(())
    }

    pub fn reorder_downloads(&self, ordered_ids: Vec<String>) -> Result<(), FfiError> {
        self.runtime
            .block_on(self.engine.reorder_downloads(&ordered_ids))?;
        Ok(())
    }

    pub fn prepare_local_download_playback_json(
        &self,
        download_id: String,
    ) -> Result<String, FfiError> {
        let bundle = self
            .runtime
            .block_on(self.engine.prepare_local_download_playback(&download_id))?;
        to_json(&bundle)
    }

    pub fn shutdown(&self) {
        self.runtime.block_on(self.engine.shutdown());
    }
}

struct NoopEventSink;
impl EventSink for NoopEventSink {
    fn emit(&self, _event: EngineEvent) {}
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{EngineEventListener, VesselMobileCore};

    struct CapturingListener {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl EngineEventListener for CapturingListener {
        fn on_event_json(&self, event_json: String) {
            self.events.lock().expect("lock").push(event_json);
        }
    }

    #[test]
    fn initializes_mobile_core_installs_plugin_and_emits_events() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("vessel-ffi-test-{nanos}"));
        let root_dir = base.join("public");
        let data_dir = base.join("private");
        let plugin_dir = base.join("dev-plugin");
        std::fs::create_dir_all(&plugin_dir).expect("create dev plugin dir");
        std::fs::write(
            plugin_dir.join("manifest.json"),
            r#"{
                "schemaVersion": 1,
                "id": "anthology",
                "name": "Anthology Mobile",
                "version": "1.0.0",
                "entry": "index.ts",
                "capabilities": ["vod", "search"],
                "contentTypes": ["anime"],
                "languages": ["en"],
                "domains": []
            }"#,
        )
        .expect("write manifest");
        std::fs::write(
            plugin_dir.join("index.ts"),
            r#"export default {
                async getHomeFeed() {
                    return {
                        sections: [{
                            id: "featured",
                            title: "Featured",
                            layout: "poster",
                            items: [{ nativeId: "show-1", type: "series", isAnime: true, title: "Gintama" }]
                        }]
                    };
                },
                async search(query) {
                    return {
                        items: [{ nativeId: "show-1", type: "series", isAnime: true, title: `Match: ${query}` }],
                        hasMore: false
                    };
                },
                async getDetails(id) {
                    return {
                        nativeId: id,
                        type: "series",
                        isAnime: true,
                        title: "Gintama",
                        episodes: [{ streamId: "ep-1", season: 1, episode: 1 }]
                    };
                },
                async getStreams(sid) {
                    return {
                        streams: [{ url: `https://cdn.anthology.example/${sid}.m3u8`, kind: "hls", label: "1080p", isDefault: true }]
                    };
                }
            };"#,
        )
        .expect("write plugin");

        let events = Arc::new(Mutex::new(Vec::new()));
        let listener = CapturingListener {
            events: Arc::clone(&events),
        };
        let core = VesselMobileCore::open(
            root_dir.to_string_lossy().into_owned(),
            data_dir.to_string_lossy().into_owned(),
            Some(Box::new(listener)),
        )
        .expect("open mobile core");

        let info: serde_json::Value =
            serde_json::from_str(&core.get_engine_info_json().expect("info")).expect("parse info");
        assert!(
            info["proxyBaseUrl"]
                .as_str()
                .unwrap_or("")
                .starts_with("http://127.0.0.1:")
        );

        let themes: serde_json::Value =
            serde_json::from_str(&core.list_themes_json().expect("themes")).expect("parse themes");
        assert_eq!(themes.as_array().map(Vec::len), Some(5));

        let install: serde_json::Value = serde_json::from_str(
            &core
                .install_source_json(plugin_dir.to_string_lossy().into_owned(), None)
                .expect("install plugin"),
        )
        .expect("parse install");
        assert_eq!(install["pluginId"].as_str(), Some("anthology"));

        let feeds: serde_json::Value =
            serde_json::from_str(&core.get_home_feeds_json(None).expect("feeds"))
                .expect("parse feeds");
        assert_eq!(
            feeds[0]["sections"][0]["items"][0]["title"].as_str(),
            Some("Gintama")
        );

        let _search_id = core
            .search_streams("gintama".into(), None, None, None)
            .expect("search");

        let mut received_search = false;
        for _ in 0..100 {
            if events
                .lock()
                .expect("lock")
                .iter()
                .any(|e| e.contains("search_partial_result") && e.contains("Match: gintama"))
            {
                received_search = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            received_search,
            "expected searchResult event emitted through UniFFI listener; got: {:?}",
            *events.lock().expect("lock")
        );

        let streams_json = core
            .get_streams_json("anthology".into(), "ep-1".into())
            .expect("get_streams");
        assert!(streams_json.contains("cdn.anthology.example/ep-1.m3u8"));

        core.shutdown();
        let _ = std::fs::remove_dir_all(&base);
    }
}
