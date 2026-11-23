//! Pure-Rust IPTV client supporting M3U/M3U-Plus, Xtream Codes, and XMLTV EPG.

pub mod m3u;
pub mod xmltv;
pub mod xtream;

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;
use vessel_domain::{StreamCandidate, StreamFlag, StreamKind, VesselError, VesselResult};

/// IPTV playlist/provider protocol kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IptvKind {
    Xtream,
    M3u,
}

/// Configured IPTV account or playlist source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IptvProfile {
    pub id: String,
    pub name: String,
    pub kind: IptvKind,
    pub server_url: String,
    pub username: Option<String>,
    pub secret_ref: Option<String>,
    pub epg_url: Option<String>,
    pub user_agent: Option<String>,
    pub last_synced_at: Option<String>,
}

/// Normalized IPTV live channel extracted from M3U or Xtream Codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IptvChannel {
    pub stream_id: String,
    pub name: String,
    pub group_title: String,
    pub logo_url: Option<String>,
    pub stream_url: String,
    pub tvg_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

/// Single parsed XMLTV EPG schedule slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpgProgram {
    pub channel_tvg_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
}

/// Active and upcoming EPG programs for one channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpgNowNext {
    pub now: Option<EpgProgram>,
    pub next: Option<EpgProgram>,
}

/// Async IPTV client fetching M3U playlists, Xtream APIs, and XMLTV feeds.
#[derive(Debug, Clone)]
pub struct IptvClient {
    http: reqwest::Client,
}

impl Default for IptvClient {
    fn default() -> Self {
        Self::new()
    }
}

impl IptvClient {
    /// Creates an [`IptvClient`] with conservative connection/request timeouts.
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { http }
    }

    async fn fetch_text(&self, url: &str, user_agent: Option<&str>) -> VesselResult<String> {
        let parsed = Url::parse(url.trim())
            .map_err(|err| VesselError::InvalidInput(format!("invalid IPTV feed URL: {err}")))?;
        match parsed.scheme() {
            "http" | "https" => {}
            other => {
                return Err(VesselError::InvalidInput(format!(
                    "unsupported IPTV URL scheme '{other}'"
                )));
            }
        }

        let mut req = self.http.get(parsed);
        if let Some(ua) = user_agent
            && !ua.trim().is_empty()
        {
            req = req.header(reqwest::header::USER_AGENT, ua.trim());
        }

        let resp = req
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("IPTV fetch failed: {err}")))?;
        if !resp.status().is_success() {
            return Err(VesselError::Network(format!(
                "IPTV server returned HTTP {}",
                resp.status()
            )));
        }
        resp.text()
            .await
            .map_err(|err| VesselError::Network(format!("failed reading IPTV response: {err}")))
    }

    /// Downloads and parses a remote M3U / M3U-Plus playlist.
    pub async fn fetch_m3u(
        &self,
        playlist_url: &str,
        user_agent: Option<&str>,
    ) -> VesselResult<Vec<IptvChannel>> {
        let text = self.fetch_text(playlist_url, user_agent).await?;
        let mut channels = m3u::parse_m3u(&text);
        if let Some(default_ua) = user_agent.map(str::trim).filter(|s| !s.is_empty()) {
            for ch in &mut channels {
                if ch.user_agent.is_none() {
                    ch.user_agent = Some(default_ua.to_string());
                }
            }
        }
        Ok(channels)
    }

    /// Fetches live channels from an Xtream Codes compatible endpoint.
    pub async fn fetch_xtream_channels(
        &self,
        server_url: &str,
        username: &str,
        password: &str,
        user_agent: Option<&str>,
    ) -> VesselResult<Vec<IptvChannel>> {
        xtream::fetch_xtream_channels(&self.http, server_url, username, password, user_agent).await
    }

    /// Downloads and streams an XMLTV feed into [`EpgProgram`] records.
    pub async fn fetch_xmltv_epg(
        &self,
        epg_url: &str,
        user_agent: Option<&str>,
    ) -> VesselResult<Vec<EpgProgram>> {
        let xml = self.fetch_text(epg_url, user_agent).await?;
        xmltv::parse_xmltv_programs(&xml)
    }
}

fn infer_stream_kind(stream_url: &str) -> StreamKind {
    let path_lower = Url::parse(stream_url)
        .map(|u| u.path().to_ascii_lowercase())
        .unwrap_or_else(|_| stream_url.to_ascii_lowercase());
    if path_lower.ends_with(".m3u8") {
        StreamKind::Hls
    } else if path_lower.ends_with(".mp4") {
        StreamKind::Mp4
    } else if path_lower.ends_with(".mkv") {
        StreamKind::Mkv
    } else {
        StreamKind::MpegTs
    }
}

/// Converts an IPTV channel into a [`StreamCandidate`] routed through Vessel's Smart Proxy.
pub fn channel_to_stream_candidate(
    channel: &IptvChannel,
    profile_user_agent: Option<&str>,
) -> StreamCandidate {
    let mut headers = BTreeMap::new();
    let effective_ua = channel
        .user_agent
        .as_deref()
        .or(profile_user_agent)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(ua) = effective_ua {
        headers.insert("User-Agent".to_string(), ua.to_string());
    }

    StreamCandidate {
        url: channel.stream_url.clone(),
        kind: infer_stream_kind(&channel.stream_url),
        quality: None,
        label: channel.name.clone(),
        language: None,
        audio_type: None,
        priority: None,
        is_default: true,
        flags: vec![StreamFlag::RequiresProxy],
        headers,
        needs_extractor: false,
        subtitles: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{IptvChannel, channel_to_stream_candidate};
    use vessel_domain::{StreamFlag, StreamKind};

    #[test]
    fn builds_proxied_stream_candidate_with_custom_user_agent() {
        let channel = IptvChannel {
            stream_id: "xtream-1".into(),
            name: "Anthology Live".into(),
            group_title: "News".into(),
            logo_url: None,
            stream_url: "https://iptv.anthology.example/live/ch1.m3u8".into(),
            tvg_id: Some("ch1".into()),
            user_agent: None,
        };

        let candidate = channel_to_stream_candidate(&channel, Some("CustomBox/1.4"));
        assert_eq!(candidate.kind, StreamKind::Hls);
        assert!(candidate.flags.contains(&StreamFlag::RequiresProxy));
        assert_eq!(
            candidate.headers.get("User-Agent").map(String::as_str),
            Some("CustomBox/1.4")
        );
    }
}
