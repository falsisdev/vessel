//! IPTV M3U/Xtream profile management, XMLTV EPG sync, FTS5 search, and channel playback.

use vessel_domain::{MediaRef, MediaType, PluginId, VesselError, VesselResult};
use vessel_iptv::{
    EpgNowNext, EpgProgram, IptvChannel, IptvKind, IptvProfile, channel_to_stream_candidate,
    xtream::xtream_xmltv_url,
};
use vessel_storage::{
    NewIptvChannel, StoredEpgNowNext, StoredEpgProgram, StoredIptvChannel, StoredIptvKind,
    StoredIptvProfile,
};

use crate::{PlayCandidateRequest, VesselEngine};

impl VesselEngine {
    pub async fn upsert_iptv_profile(
        &self,
        profile: IptvProfile,
        password: Option<&str>,
    ) -> VesselResult<StoredIptvProfile> {
        let secret_ref = match password {
            Some(pass) => {
                let key = format!("iptv:{}:password", profile.id);
                self.secrets.set(&key, pass).await?;
                Some(key)
            }
            None => profile.secret_ref,
        };
        let stored = StoredIptvProfile {
            id: profile.id,
            name: profile.name,
            kind: match profile.kind {
                IptvKind::Xtream => StoredIptvKind::Xtream,
                IptvKind::M3u => StoredIptvKind::M3u,
            },
            server_url: profile.server_url,
            username: profile.username,
            secret_ref,
            epg_url: profile.epg_url,
            user_agent: profile.user_agent,
            last_synced_at: profile.last_synced_at,
        };
        self.storage.upsert_iptv_profile(&stored).await?;
        Ok(stored)
    }

    /// Lists configured IPTV profiles.
    pub async fn list_iptv_profiles(&self) -> VesselResult<Vec<StoredIptvProfile>> {
        self.storage.list_iptv_profiles().await
    }

    /// Deletes an IPTV profile, its channels/EPG (via cascade), and any stored password secret.
    pub async fn delete_iptv_profile(&self, profile_id: &str) -> VesselResult<()> {
        let profiles = self.storage.list_iptv_profiles().await?;
        if let Some(target) = profiles.into_iter().find(|p| p.id == profile_id)
            && let Some(secret_ref) = target.secret_ref
        {
            self.secrets.delete(&secret_ref).await?;
        }
        self.storage.delete_iptv_profile(profile_id).await
    }

    /// Syncs channels for `profile_id` from its M3U or Xtream Codes source, preserving favorites.
    pub async fn sync_iptv_profile(&self, profile_id: &str) -> VesselResult<usize> {
        let profile = self
            .storage
            .list_iptv_profiles()
            .await?
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| {
                VesselError::NotFound(format!("IPTV profile '{profile_id}' not found"))
            })?;

        let channels = match profile.kind {
            StoredIptvKind::M3u => {
                self.iptv
                    .fetch_m3u(&profile.server_url, profile.user_agent.as_deref())
                    .await?
            }
            StoredIptvKind::Xtream => {
                let username = profile.username.as_deref().unwrap_or("");
                let password = match profile.secret_ref.as_deref() {
                    Some(key) => self.secrets.get(key).await?.unwrap_or_default(),
                    None => String::new(),
                };
                self.iptv
                    .fetch_xtream_channels(
                        &profile.server_url,
                        username,
                        &password,
                        profile.user_agent.as_deref(),
                    )
                    .await?
            }
        };

        let converted: Vec<NewIptvChannel> = channels
            .into_iter()
            .map(|ch| NewIptvChannel {
                stream_id: ch.stream_id,
                name: ch.name,
                group_title: ch.group_title,
                logo_url: ch.logo_url,
                stream_url: ch.stream_url,
                tvg_id: ch.tvg_id,
            })
            .collect();

        self.storage
            .replace_iptv_channels(profile_id, converted)
            .await
    }

    /// Downloads and stores XMLTV EPG entries for `profile_id`.
    pub async fn sync_iptv_epg(&self, profile_id: &str) -> VesselResult<usize> {
        let profile = self
            .storage
            .list_iptv_profiles()
            .await?
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| {
                VesselError::NotFound(format!("IPTV profile '{profile_id}' not found"))
            })?;

        let epg_url = match profile.epg_url {
            Some(url) if !url.trim().is_empty() => url,
            _ if profile.kind == StoredIptvKind::Xtream => {
                let username = profile.username.as_deref().unwrap_or_default();
                let password = match profile.secret_ref.as_deref() {
                    Some(key) => self.secrets.get(key).await?.unwrap_or_default(),
                    None => String::new(),
                };
                xtream_xmltv_url(&profile.server_url, username, &password)?
            }
            _ => return Ok(0),
        };

        let programs = self
            .iptv
            .fetch_xmltv_epg(&epg_url, profile.user_agent.as_deref())
            .await?;
        let stored: Vec<StoredEpgProgram> = programs
            .into_iter()
            .map(|p| StoredEpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            })
            .collect();
        self.storage.replace_epg_programs(profile_id, stored).await
    }

    /// Searches synced IPTV channels using SQLite FTS5 prefix matching and optional filters.
    pub async fn search_iptv_channels(
        &self,
        profile_id: Option<&str>,
        query: Option<&str>,
        group_title: Option<&str>,
        favorites_only: bool,
        limit: usize,
    ) -> VesselResult<Vec<StoredIptvChannel>> {
        self.storage
            .search_iptv_channels(profile_id, query, group_title, favorites_only, limit)
            .await
    }

    /// Lists sorted IPTV category group names for `profile_id`.
    pub async fn list_iptv_groups(&self, profile_id: &str) -> VesselResult<Vec<String>> {
        self.storage.list_iptv_groups(profile_id).await
    }

    /// Marks or unmarks an IPTV channel row as favorite.
    pub async fn set_iptv_favorite(&self, channel_id: i64, is_favorite: bool) -> VesselResult<()> {
        self.storage
            .set_iptv_favorite(channel_id, is_favorite)
            .await
    }

    /// Deletes expired XMLTV EPG entries ending at or before `before_ts`.
    pub async fn prune_expired_epg(&self, before_ts: i64) -> VesselResult<usize> {
        self.storage.prune_expired_epg(before_ts).await
    }

    /// Returns active (`now`) and upcoming (`next`) programs for a channel `tvg_id`.
    pub async fn get_epg_now_next(
        &self,
        profile_id: &str,
        channel_tvg_id: &str,
        now_ts: i64,
    ) -> VesselResult<EpgNowNext> {
        let StoredEpgNowNext { now, next } = self
            .storage
            .get_epg_now_next(profile_id, channel_tvg_id, now_ts)
            .await?;
        Ok(EpgNowNext {
            now: now.map(|p| EpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            }),
            next: next.map(|p| EpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            }),
        })
    }

    /// Routes an IPTV live channel through Smart Proxy (preserving custom `User-Agent`) and plays it.
    pub async fn play_iptv_channel(
        &self,
        channel: &StoredIptvChannel,
        profile_user_agent: Option<&str>,
    ) -> VesselResult<String> {
        let iptv_ch = IptvChannel {
            stream_id: channel.stream_id.clone(),
            name: channel.name.clone(),
            group_title: channel.group_title.clone(),
            logo_url: channel.logo_url.clone(),
            stream_url: channel.stream_url.clone(),
            tvg_id: channel.tvg_id.clone(),
            user_agent: None,
        };
        let candidate = channel_to_stream_candidate(&iptv_ch, profile_user_agent);
        let media = MediaRef::new(PluginId::new("iptv")?, channel.stream_id.clone())?;
        self.play_candidate(PlayCandidateRequest {
            media,
            canonical_id: None,
            season: 0,
            episode: 0,
            absolute_episode: None,
            media_type: MediaType::Movie,
            candidate,
            extra_subtitles: Vec::new(),
            start_position_ms: Some(0),
            title: Some(channel.name.clone()),
        })
        .await
    }
}
