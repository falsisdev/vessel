//! Plugin installation, updates, domain permissions, and bootstrap orchestration.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vessel_domain::{
    DomainGrantSource, EngineEvent, LogLevel, PluginId, RepoPluginEntry, VesselError, VesselResult,
};
use vessel_plugin_rt::{
    LoadedPackage, ResolvedSource, SourceTrustBadge, load_plugin_dir, load_plugin_package,
    pack_plugin_package,
};
use vessel_storage::StoredPlugin;

use crate::{
    BatchUpdateFailure, BatchUpdateResult, PluginUpdateInfo, UpdatedPluginInfo, VesselEngine,
    builtins,
};

impl VesselEngine {
    pub async fn resolve_source(&self, raw_input: &str) -> VesselResult<ResolvedSource> {
        self.source_resolver.resolve(raw_input).await
    }

    /// Persists a `LoadedPackage`, records manifest-declared domains, and registers it in the pool.
    pub async fn install_package(
        &self,
        package: LoadedPackage,
        repo_id: Option<String>,
        source_type: &str,
        local_path: &str,
    ) -> VesselResult<()> {
        self.install_package_with_source(package, repo_id, source_type, None, local_path)
            .await
    }

    /// Persists a `LoadedPackage` along with its origin `source_url`, preserving existing settings,
    /// user-approved domains, and enabled state on in-place updates.
    pub async fn install_package_with_source(
        &self,
        package: LoadedPackage,
        repo_id: Option<String>,
        source_type: &str,
        source_url: Option<String>,
        local_path: &str,
    ) -> VesselResult<()> {
        package.manifest.validate()?;
        let plugin_id = package.manifest.id.clone();
        let existing = self.storage.get_plugin(&plugin_id).await?;
        let preserve_enabled = existing.as_ref().map(|p| p.enabled).unwrap_or(true);
        if existing.is_some() {
            self.clear_plugin_caches(&plugin_id).await;
        }

        let stored = StoredPlugin {
            manifest: package.manifest.clone(),
            repo_id: repo_id.or_else(|| existing.and_then(|p| p.repo_id)),
            source_type: source_type.to_owned(),
            source_url,
            local_path: local_path.to_owned(),
            sha256: Some(package.sha256.clone()),
            enabled: preserve_enabled,
            allow_dynamic_domains: package.manifest.allow_any_https,
            installed_at: "1970-01-01T00:00:00Z".into(),
            updated_at: "1970-01-01T00:00:00Z".into(),
        };
        // Persist timestamp consistently using SQLite CURRENT / now when upserting domain grants below.
        self.storage.upsert_plugin(stored).await?;

        for domain in &package.manifest.domains {
            let trimmed = domain.trim();
            if !trimmed.is_empty() {
                self.storage
                    .grant_domain(&plugin_id, trimmed, DomainGrantSource::Manifest)
                    .await?;
            }
        }

        let extra_domains = self.storage.list_allowed_domains(&plugin_id).await?;
        self.pool
            .register(
                package.manifest,
                Arc::new(package.bundle),
                extra_domains,
                preserve_enabled,
            )
            .await?;

        if let Ok(settings) = self.get_plugin_settings(&plugin_id).await {
            let _ = self.pool.set_settings(&plugin_id, settings).await;
        }
        self.sync_community_debrid_providers().await;

        Ok(())
    }

    /// Resolves `raw_input` and installs a single `.plugin.vessel` or open directory package directly.
    pub async fn install_package_from_source(&self, raw_input: &str) -> VesselResult<PluginId> {
        match self.source_resolver.resolve(raw_input).await? {
            ResolvedSource::Package {
                resolved_url,
                package,
                ..
            } => {
                let id = package.manifest.id.clone();
                self.install_package(package, None, "source", &resolved_url)
                    .await?;
                Ok(id)
            }
            ResolvedSource::Repository { repo, .. } => Err(VesselError::InvalidInput(format!(
                "source '{}' resolved to repository '{}' rather than a single plugin package",
                raw_input, repo.name
            ))),
        }
    }

    /// Approves an additional HTTPS domain for `plugin_id` (optionally persisting permanently in SQLite).
    pub async fn grant_plugin_domain(
        &self,
        plugin_id: &PluginId,
        domain: &str,
        permanent: bool,
    ) -> VesselResult<()> {
        self.pool.allow_domain(plugin_id, domain).await?;
        if permanent {
            self.storage
                .grant_domain(plugin_id, domain, DomainGrantSource::UserPermanent)
                .await?;
        }
        Ok(())
    }

    /// Lists all persisted plugins installed in SQLite.
    pub async fn list_plugins(&self) -> VesselResult<Vec<StoredPlugin>> {
        self.storage.list_plugins().await
    }

    /// Lists active (enabled) plugin IDs currently registered in the pool.
    pub async fn enabled_plugin_ids(&self) -> Vec<PluginId> {
        self.pool.enabled_plugin_ids().await
    }

    /// Lists approved HTTPS domains for `plugin_id`.
    pub async fn list_allowed_domains(&self, plugin_id: &PluginId) -> VesselResult<Vec<String>> {
        self.storage.list_allowed_domains(plugin_id).await
    }

    /// Enables or disables an installed plugin across SQLite and the warm QuickJS pool.
    pub async fn set_plugin_enabled(
        &self,
        plugin_id: &PluginId,
        enabled: bool,
    ) -> VesselResult<()> {
        self.clear_plugin_caches(plugin_id).await;
        self.storage.set_plugin_enabled(plugin_id, enabled).await?;
        self.pool.set_enabled(plugin_id, enabled).await?;
        self.sync_community_debrid_providers().await;
        Ok(())
    }

    /// Resolves `raw_input` and persists the package into `plugins_dir` (or tracks local Dev Mode dir),
    /// tagging an optional `repo_id` group label when installed from a repository.
    pub async fn install_package_persisted_with_repo(
        &self,
        plugins_dir: &Path,
        raw_input: &str,
        repo_id: Option<String>,
    ) -> VesselResult<(PluginId, SourceTrustBadge)> {
        let inferred_repo = repo_id.or_else(|| infer_repo_group(raw_input));
        match self.source_resolver.resolve(raw_input).await? {
            ResolvedSource::Package {
                resolved_url,
                package,
                badge,
            } => {
                let id = package.manifest.id.clone();
                let origin_url = if raw_input.trim().is_empty() {
                    resolved_url.clone()
                } else {
                    resolved_url.clone()
                };
                let dev_path = Path::new(&resolved_url);
                if dev_path.is_dir() {
                    self.install_package_with_source(
                        package,
                        inferred_repo,
                        "dev_dir",
                        Some(origin_url),
                        &resolved_url,
                    )
                    .await?;
                } else {
                    std::fs::create_dir_all(plugins_dir).map_err(|err| {
                        VesselError::Storage(format!("create plugins directory: {err}"))
                    })?;
                    let pkg_path = plugins_dir.join(format!("{}.plugin.vessel", id.as_str()));
                    let bytes = pack_plugin_package(
                        &package.manifest,
                        package.bundle.files(),
                        package.icon_png.as_deref(),
                    )?;
                    std::fs::write(&pkg_path, bytes).map_err(|err| {
                        VesselError::Storage(format!("write plugin archive: {err}"))
                    })?;
                    let local_path = pkg_path.to_string_lossy().into_owned();
                    self.install_package_with_source(
                        package,
                        inferred_repo,
                        "package",
                        Some(origin_url),
                        &local_path,
                    )
                    .await?;
                }
                Ok((id, badge))
            }
            ResolvedSource::Repository { repo, .. } => Err(VesselError::InvalidInput(format!(
                "source '{}' resolved to repository '{}' rather than a single plugin package",
                raw_input, repo.name
            ))),
        }
    }

    /// Resolves `raw_input` and persists the package into `plugins_dir` (or tracks local Dev Mode dir).
    pub async fn install_package_persisted(
        &self,
        plugins_dir: &Path,
        raw_input: &str,
    ) -> VesselResult<(PluginId, SourceTrustBadge)> {
        self.install_package_persisted_with_repo(plugins_dir, raw_input, None)
            .await
    }

    /// Ensures built-in first-party plugins are persisted in SQLite and registered in the pool,
    /// preserving the user's enabled toggle, settings, and granted domains across app updates.
    pub async fn ensure_builtin_plugins(&self) -> VesselResult<usize> {
        let packages = builtins::builtin_packages()?;
        let mut count = 0usize;
        for pkg in packages {
            let plugin_id = pkg.manifest.id.clone();
            let local_path = format!("builtin://{}", plugin_id.as_str());
            self.install_package_with_source(
                pkg,
                Some("builtin".to_owned()),
                builtins::BUILTIN_SOURCE_TYPE,
                None,
                &local_path,
            )
            .await?;
            count += 1;
        }
        Ok(count)
    }

    /// Reloads all persisted plugins from SQLite and their local files/directories into the pool.
    pub async fn bootstrap_plugins(&self) -> VesselResult<usize> {
        let mut loaded_count = self.ensure_builtin_plugins().await?;
        let plugins = self.storage.list_plugins().await?;
        for stored in plugins {
            if stored.source_type == builtins::BUILTIN_SOURCE_TYPE {
                continue;
            }
            let path = PathBuf::from(&stored.local_path);
            let pkg_res = if stored.source_type == "dev_dir" || path.is_dir() {
                load_plugin_dir(&path)
            } else {
                std::fs::read(&path)
                    .map_err(|err| {
                        VesselError::Storage(format!(
                            "read plugin archive '{}': {err}",
                            path.display()
                        ))
                    })
                    .and_then(|bytes| load_plugin_package(&bytes, None))
            };

            match pkg_res {
                Ok(pkg) => {
                    let plugin_id = pkg.manifest.id.clone();
                    let extra_domains = self.storage.list_allowed_domains(&plugin_id).await?;
                    if let Err(err) = self
                        .pool
                        .register(
                            pkg.manifest,
                            Arc::new(pkg.bundle),
                            extra_domains,
                            stored.enabled,
                        )
                        .await
                    {
                        self.event_sink.emit(EngineEvent::PluginLog {
                            plugin_id,
                            level: LogLevel::Warn,
                            message: format!("Failed registering plugin on startup: {err}"),
                        });
                    } else {
                        if let Ok(settings) = self.get_plugin_settings(&plugin_id).await {
                            let _ = self.pool.set_settings(&plugin_id, settings).await;
                        }
                        loaded_count += 1;
                    }
                }
                Err(err) => {
                    self.event_sink.emit(EngineEvent::PluginLog {
                        plugin_id: stored.manifest.id,
                        level: LogLevel::Warn,
                        message: format!("Failed loading plugin on startup: {err}"),
                    });
                }
            }
        }
        self.sync_community_debrid_providers().await;
        Ok(loaded_count)
    }

    /// Uninstalls `plugin_id`, removing packaged archives (never deleting Dev Mode folders) and DB rows.
    pub async fn uninstall_plugin(&self, plugin_id: &PluginId) -> VesselResult<()> {
        let existing = self.storage.get_plugin(plugin_id).await?;
        if existing
            .as_ref()
            .is_some_and(|p| p.source_type == builtins::BUILTIN_SOURCE_TYPE)
        {
            return Err(VesselError::InvalidInput(format!(
                "built-in plugin '{}' cannot be uninstalled; disable it instead",
                plugin_id.as_str()
            )));
        }
        self.clear_plugin_caches(plugin_id).await;
        self.pool.unregister(plugin_id).await;
        if let Some(stored) = existing
            && stored.source_type != "dev_dir"
        {
            let path = Path::new(&stored.local_path);
            if path.is_file() {
                let _ = std::fs::remove_file(path);
            }
        }
        self.storage.remove_plugin(plugin_id).await?;
        self.sync_community_debrid_providers().await;
        Ok(())
    }

    async fn resolve_repo_entries_cached<'a>(
        &self,
        repo_key: &str,
        cache: &'a mut HashMap<String, Result<Vec<RepoPluginEntry>, String>>,
    ) -> Result<&'a [RepoPluginEntry], String> {
        if !cache.contains_key(repo_key) {
            let fetched = match self.source_resolver.resolve(repo_key).await {
                Ok(ResolvedSource::Repository { repo, .. }) => Ok(repo.plugins),
                Ok(ResolvedSource::Package { .. }) => Err(format!(
                    "source '{repo_key}' resolved to a package instead of a repo"
                )),
                Err(err) => Err(err.to_string()),
            };
            cache.insert(repo_key.to_owned(), fetched);
        }
        match cache.get(repo_key) {
            Some(Ok(list)) => Ok(list.as_slice()),
            Some(Err(err)) => Err(err.clone()),
            None => Err("missing cached repo".into()),
        }
    }

    /// Checks all installed plugins for newer versions without modifying any settings or domain grants.
    pub async fn check_plugin_updates(&self) -> VesselResult<Vec<PluginUpdateInfo>> {
        let installed = self.storage.list_plugins().await?;
        let mut repo_cache: HashMap<String, Result<Vec<RepoPluginEntry>, String>> = HashMap::new();
        let mut out = Vec::with_capacity(installed.len());

        for plugin in installed {
            let pid = plugin.manifest.id.as_str().to_owned();
            let current_version = plugin.manifest.version.clone();

            if plugin.source_type == builtins::BUILTIN_SOURCE_TYPE {
                out.push(PluginUpdateInfo {
                    plugin_id: pid,
                    current_version: current_version.clone(),
                    latest_version: Some(current_version),
                    update_available: false,
                    source_url: None,
                    error: None,
                });
                continue;
            }

            if let Some(repo_key) = infer_repo_source_for_plugin(&plugin) {
                match self
                    .resolve_repo_entries_cached(&repo_key, &mut repo_cache)
                    .await
                {
                    Ok(entries) => {
                        if let Some(entry) = entries.iter().find(|e| e.id.as_str() == pid) {
                            let source_url = entry
                                .download_url
                                .clone()
                                .or_else(|| entry.source_path.clone())
                                .or_else(|| plugin.source_url.clone());
                            let update_available =
                                is_newer_version(&current_version, &entry.version);
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: Some(entry.version.clone()),
                                update_available,
                                source_url,
                                error: None,
                            });
                            continue;
                        }
                    }
                    Err(err) => {
                        if plugin.source_url.is_none() {
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: None,
                                update_available: false,
                                source_url: None,
                                error: Some(err),
                            });
                            continue;
                        }
                    }
                }
            }

            if let Some(ref src) = plugin.source_url {
                match self.source_resolver.resolve(src).await {
                    Ok(ResolvedSource::Package { package, .. }) => {
                        let latest = package.manifest.version;
                        let update_available = is_newer_version(&current_version, &latest);
                        out.push(PluginUpdateInfo {
                            plugin_id: pid,
                            current_version,
                            latest_version: Some(latest),
                            update_available,
                            source_url: Some(src.clone()),
                            error: None,
                        });
                    }
                    Ok(ResolvedSource::Repository { repo, .. }) => {
                        if let Some(entry) = repo.plugins.iter().find(|e| e.id.as_str() == pid) {
                            let target = entry
                                .download_url
                                .clone()
                                .or_else(|| entry.source_path.clone())
                                .or_else(|| Some(src.clone()));
                            let update_available =
                                is_newer_version(&current_version, &entry.version);
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: Some(entry.version.clone()),
                                update_available,
                                source_url: target,
                                error: None,
                            });
                        } else {
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: None,
                                update_available: false,
                                source_url: Some(src.clone()),
                                error: None,
                            });
                        }
                    }
                    Err(err) => {
                        out.push(PluginUpdateInfo {
                            plugin_id: pid,
                            current_version,
                            latest_version: None,
                            update_available: false,
                            source_url: Some(src.clone()),
                            error: Some(err.to_string()),
                        });
                    }
                }
            } else {
                out.push(PluginUpdateInfo {
                    plugin_id: pid,
                    current_version,
                    latest_version: None,
                    update_available: false,
                    source_url: None,
                    error: None,
                });
            }
        }

        Ok(out)
    }

    /// Updates a single installed plugin in-place, preserving user settings, domain grants, and enabled state.
    pub async fn update_plugin(
        &self,
        plugins_dir: &Path,
        plugin_id: &PluginId,
    ) -> VesselResult<(PluginId, SourceTrustBadge, String)> {
        let Some(stored) = self.storage.get_plugin(plugin_id).await? else {
            return Err(VesselError::NotFound(format!(
                "plugin '{}'",
                plugin_id.as_str()
            )));
        };

        if stored.source_type == builtins::BUILTIN_SOURCE_TYPE {
            return Err(VesselError::InvalidInput(format!(
                "built-in plugin '{}' updates automatically with Vessel",
                plugin_id.as_str()
            )));
        }

        let mut target_source = if stored.source_type == "dev_dir" {
            Some(stored.local_path.clone())
        } else {
            None
        };

        if target_source.is_none()
            && let Some(repo_key) = infer_repo_source_for_plugin(&stored)
        {
            let mut cache = HashMap::new();
            if let Ok(entries) = self
                .resolve_repo_entries_cached(&repo_key, &mut cache)
                .await
                && let Some(entry) = entries.iter().find(|e| e.id == *plugin_id)
            {
                target_source = entry
                    .download_url
                    .clone()
                    .or_else(|| entry.source_path.clone());
            }
        }

        if target_source.is_none() {
            target_source = stored.source_url.clone();
        }

        let Some(raw_target) = target_source.filter(|s| !s.trim().is_empty()) else {
            return Err(VesselError::InvalidInput(format!(
                "plugin '{}' has no recorded update source; reinstall once from a repository or URL",
                plugin_id.as_str()
            )));
        };

        let (id, badge) = self
            .install_package_persisted_with_repo(plugins_dir, &raw_target, stored.repo_id)
            .await?;
        let new_version = self
            .storage
            .get_plugin(&id)
            .await?
            .map(|p| p.manifest.version)
            .unwrap_or(stored.manifest.version);

        Ok((id, badge, new_version))
    }

    /// Checks and updates all outdated plugins in-place.
    pub async fn update_all_plugins(&self, plugins_dir: &Path) -> VesselResult<BatchUpdateResult> {
        let checks = self.check_plugin_updates().await?;
        let checked = checks.len();
        let mut updated = Vec::new();
        let mut failed = Vec::new();

        for item in checks {
            if !item.update_available {
                continue;
            }
            let Some(target) = item.source_url else {
                continue;
            };
            let Ok(pid) = PluginId::new(&item.plugin_id) else {
                continue;
            };
            let existing_repo = self.storage.get_plugin(&pid).await?.and_then(|p| p.repo_id);
            match self
                .install_package_persisted_with_repo(plugins_dir, &target, existing_repo)
                .await
            {
                Ok((id, badge)) => {
                    let version = self
                        .storage
                        .get_plugin(&id)
                        .await?
                        .map(|p| p.manifest.version)
                        .or(item.latest_version)
                        .unwrap_or(item.current_version);
                    updated.push(UpdatedPluginInfo {
                        plugin_id: id.to_string(),
                        version,
                        badge: format!("{badge:?}").to_ascii_lowercase(),
                    });
                }
                Err(err) => {
                    failed.push(BatchUpdateFailure {
                        plugin_id: item.plugin_id,
                        error: err.to_string(),
                    });
                }
            }
        }

        Ok(BatchUpdateResult {
            checked,
            updated,
            failed,
        })
    }

}

fn infer_repo_group(raw_input: &str) -> Option<String> {
    let clean = raw_input
        .trim()
        .strip_prefix("nuvio-plugin://")
        .unwrap_or(raw_input.trim());
    if let Some(rest) = clean.strip_prefix("gh:") {
        let mut parts = rest.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            let repo_clean = repo.split('@').next().unwrap_or(repo);
            return Some(format!("{owner}/{repo_clean}"));
        }
    }
    if let Some(after_raw) = clean.strip_prefix("https://raw.githubusercontent.com/") {
        let mut parts = after_raw.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            return Some(format!("{owner}/{repo}"));
        }
    }
    if clean.starts_with("stremio://") || clean.ends_with("/manifest.json") {
        return Some("Stremio & External".into());
    }
    None
}

fn infer_repo_source_for_plugin(stored: &StoredPlugin) -> Option<String> {
    if let Some(ref rid) = stored.repo_id {
        let trimmed = rid.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower == "anthology (vessel)" || lower == "anthology-vessel" || lower == "tr" {
            return Some("anthology".into());
        }
        if lower == "anthology" {
            return Some("nuvio".into());
        }
        if trimmed.contains('/') && !trimmed.contains(' ') {
            return Some(format!("gh:{trimmed}"));
        }
    }
    if let Some(ref src) = stored.source_url {
        if src.contains("/falsisdev/anthology-vessel/") {
            return Some("anthology".into());
        }
        if src.contains("/falsisdev/anthology/") {
            return Some("nuvio".into());
        }
    }
    None
}

fn parse_numeric_version(raw: &str) -> Vec<u64> {
    let clean = raw.trim().trim_start_matches(['v', 'V']);
    let core = clean.split(['-', '+']).next().unwrap_or(clean);
    core.split('.')
        .filter_map(|seg| seg.trim().parse::<u64>().ok())
        .collect()
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let cur_clean = current.trim().trim_start_matches(['v', 'V']);
    let lat_clean = latest.trim().trim_start_matches(['v', 'V']);
    if cur_clean == lat_clean || lat_clean.is_empty() {
        return false;
    }
    let a = parse_numeric_version(cur_clean);
    let b = parse_numeric_version(lat_clean);
    if !a.is_empty() && !b.is_empty() {
        let max_len = a.len().max(b.len());
        for i in 0..max_len {
            let av = *a.get(i).unwrap_or(&0);
            let bv = *b.get(i).unwrap_or(&0);
            if bv > av {
                return true;
            }
            if bv < av {
                return false;
            }
        }
        return false;
    }
    lat_clean != cur_clean
}
