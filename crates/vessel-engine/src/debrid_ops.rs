//! Built-in & community Debrid orchestration and virtual metadata synthesis.

use std::sync::Arc;

use vessel_domain::{
    DebridAccountStatus, DebridCloudItem, DebridPreferences, DebridProvider, DebridProviderKind,
    DebridResolution, ExternalIds, MediaDetail, MediaType, PluginCapability, PluginId,
    RealDebridDeviceAuthPoll, RealDebridDeviceAuthStart, ResolveDebridRequest, ResolvedHttpLink,
    StreamCandidate, VesselResult,
};
use vessel_plugin_rt::QuickJsDebridProvider;

use crate::{
    SIMKL_CLIENT_ID_SECRET, TMDB_API_KEY_SECRET, VesselEngine, apply_canonical_override_hint,
};

impl VesselEngine {
    /// Synchronizes enabled `PluginCapability::Debrid` QuickJS community plugins into `DebridManager`.
    pub async fn sync_community_debrid_providers(&self) {
        let ids = self
            .pool
            .enabled_plugin_ids_with_any(&[PluginCapability::Debrid])
            .await;
        let mut community: Vec<Arc<dyn DebridProvider>> = Vec::with_capacity(ids.len());
        for pid in ids {
            if let Ok(prov) = self.pool.get_or_load(&pid).await {
                community.push(Arc::new(QuickJsDebridProvider::new(prov)));
            }
        }
        self.debrid.set_community_providers(community).await;
    }

    /// Reads persisted Debrid priority order, `only_cached`, and `auto_fallback` preferences.
    pub async fn get_debrid_preferences(&self) -> VesselResult<DebridPreferences> {
        Ok(self
            .storage
            .get_json_setting::<DebridPreferences>("debrid.preferences")
            .await?
            .unwrap_or_default())
    }

    /// Persists Debrid priority order and routing preferences.
    pub async fn set_debrid_preferences(
        &self,
        prefs: DebridPreferences,
    ) -> VesselResult<DebridPreferences> {
        self.storage
            .set_json_setting("debrid.preferences", &prefs)
            .await?;
        Ok(prefs)
    }

    /// Saves a built-in Debrid API key into `SecretStore`.
    pub async fn save_debrid_api_key(
        &self,
        provider: DebridProviderKind,
        api_key: &str,
    ) -> VesselResult<()> {
        self.debrid.save_builtin_api_key(provider, api_key).await
    }

    /// Removes stored credentials for a built-in Debrid provider.
    pub async fn remove_debrid_credentials(&self, provider: DebridProviderKind) -> VesselResult<()> {
        self.debrid.remove_builtin_credentials(provider).await
    }

    pub async fn start_real_debrid_device_auth(&self) -> VesselResult<RealDebridDeviceAuthStart> {
        self.debrid.start_real_debrid_device_auth().await
    }

    pub async fn poll_real_debrid_device_auth(
        &self,
        device_code: &str,
    ) -> VesselResult<RealDebridDeviceAuthPoll> {
        self.debrid.poll_real_debrid_device_auth(device_code).await
    }

    /// Returns live account verification and quota metadata for all built-in + community Debrid providers.
    pub async fn get_debrid_accounts_status(&self) -> VesselResult<Vec<DebridAccountStatus>> {
        self.sync_community_debrid_providers().await;
        let prefs = self.get_debrid_preferences().await?;
        Ok(self.debrid.accounts_status(&prefs).await)
    }

    /// Probes bulk Debrid instant cache status for torrent candidates.
    pub async fn probe_torrent_cache(
        &self,
        mut candidates: Vec<StreamCandidate>,
    ) -> VesselResult<Vec<StreamCandidate>> {
        self.sync_community_debrid_providers().await;
        let prefs = self.get_debrid_preferences().await?;
        self.debrid.probe_cache(&mut candidates, &prefs).await;
        Ok(candidates)
    }

    /// Resolves a torrent magnet/info-hash candidate via prioritized Debrid providers.
    pub async fn resolve_torrent_candidate(
        &self,
        req: ResolveDebridRequest,
    ) -> VesselResult<DebridResolution> {
        self.sync_community_debrid_providers().await;
        let prefs = self.get_debrid_preferences().await?;
        self.debrid.resolve_candidate(&req, &prefs).await
    }

    /// Lists active and completed transfers across all configured Debrid cloud accounts.
    pub async fn list_debrid_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        self.sync_community_debrid_providers().await;
        let prefs = self.get_debrid_preferences().await?;
        Ok(self.debrid.list_cloud(&prefs).await)
    }

    /// Deletes a torrent transfer from the specified Debrid provider's cloud queue.
    pub async fn delete_debrid_cloud(&self, target: &str, torrent_id: &str) -> VesselResult<()> {
        self.sync_community_debrid_providers().await;
        self.debrid.delete_cloud(target, torrent_id).await
    }

    /// Unrestricts a completed Debrid cloud link into a playable direct HTTPS URL.
    pub async fn unrestrict_debrid_cloud_link(
        &self,
        target: &str,
        link: &str,
    ) -> VesselResult<ResolvedHttpLink> {
        self.sync_community_debrid_providers().await;
        self.debrid.unrestrict_cloud_link(target, link).await
    }

    /// Enables or disables automatic IMDb stream lookup for a `Stream` plugin (e.g. Torrentio).
    pub async fn set_stream_addon_auto_enabled(
        &self,
        plugin_id: &PluginId,
        enabled: bool,
    ) -> VesselResult<()> {
        let mut disabled = self.list_disabled_stream_addons().await?;
        let pid = plugin_id.as_str().to_owned();
        if enabled {
            disabled.retain(|x| x != &pid);
        } else if !disabled.contains(&pid) {
            disabled.push(pid);
        }
        self.storage
            .set_json_setting("stream_addons.disabled", &disabled)
            .await
    }

    /// Lists plugin IDs opted out of automatic IMDb stream addon aggregation.
    pub async fn list_disabled_stream_addons(&self) -> VesselResult<Vec<String>> {
        Ok(self
            .storage
            .get_json_setting::<Vec<String>>("stream_addons.disabled")
            .await?
            .unwrap_or_default())
    }

    /// Builds a virtual `MediaDetail` backed by Cinemeta/TMDB/AniList so stream-only addons (e.g. Torrentio)
    /// work seamlessly even without any catalog plugin installed.
    pub async fn get_virtual_metadata_detail(
        &self,
        canonical_id: &str,
        title_hint: Option<&str>,
        media_type_hint: Option<MediaType>,
        preferred_lang: &str,
    ) -> VesselResult<MediaDetail> {
        let mut stub = MediaDetail {
            native_id: format!("virtual::{canonical_id}"),
            plugin_id: Some(PluginId::new("vessel.metadata")?),
            media_type: media_type_hint.unwrap_or(MediaType::Movie),
            is_anime: false,
            title: title_hint
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(canonical_id)
                .to_owned(),
            original_title: None,
            year: None,
            synopsis: None,
            poster: None,
            backdrop: None,
            genres: Vec::new(),
            rating: None,
            status: None,
            content_rating: vessel_domain::ContentRating::Sfw,
            external_ids: ExternalIds::default(),
            seasons: None,
            episodes: Vec::new(),
            relations: Vec::new(),
        };
        apply_canonical_override_hint(&mut stub, canonical_id);

        let tmdb_key = self.secrets.get(TMDB_API_KEY_SECRET).await?;
        let simkl_key = self.secrets.get(SIMKL_CLIENT_ID_SECRET).await?;
        let outcome = self
            .metadata
            .enrich_detail_with_keys(
                &stub,
                preferred_lang,
                tmdb_key.as_deref(),
                simkl_key.as_deref(),
                true,
            )
            .await?;

        let mut detail = outcome.detail;
        detail.plugin_id = Some(PluginId::new("vessel.metadata")?);
        detail.native_id = format!("virtual::{canonical_id}");

        let imdb_tag = detail.external_ids.imdb.clone().unwrap_or_default();
        if detail.episodes.is_empty() {
            detail.episodes.push(vessel_domain::Episode {
                stream_id: if imdb_tag.is_empty() {
                    format!("virtual::{canonical_id}::1::1")
                } else {
                    format!("virtual::{imdb_tag}::1::1")
                },
                season: 1,
                episode: 1,
                absolute_episode: None,
                title: Some(detail.title.clone()),
                overview: detail.synopsis.clone(),
                thumbnail: detail.backdrop.clone().or_else(|| detail.poster.clone()),
                air_date: None,
                tags: Vec::new(),
            });
        } else {
            for ep in &mut detail.episodes {
                if ep.stream_id.trim().is_empty() {
                    ep.stream_id = if imdb_tag.is_empty() {
                        format!("virtual::{canonical_id}::{}::{}", ep.season, ep.episode)
                    } else {
                        format!("virtual::{imdb_tag}::{}::{}", ep.season, ep.episode)
                    };
                }
            }
        }
        Ok(detail)
    }
}
