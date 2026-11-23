//! Xtream Codes HTTP client (`player_api.php`) for live categories and streams.

use std::collections::HashMap;

use serde::Deserialize;
use url::Url;
use vessel_domain::{VesselError, VesselResult};

use crate::IptvChannel;

const DEFAULT_XTREAM_UA: &str = "IPTVSmartersPro";

#[derive(Debug, Deserialize)]
struct XtreamCategory {
    category_id: serde_json::Value,
    #[serde(default)]
    category_name: String,
}

#[derive(Debug, Deserialize)]
struct XtreamLiveStream {
    stream_id: serde_json::Value,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    stream_icon: Option<String>,
    #[serde(default)]
    epg_channel_id: Option<String>,
    #[serde(default)]
    category_id: Option<serde_json::Value>,
}

fn json_id_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.trim().to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn non_empty_opt(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn resolve_icon_url(base: &str, value: Option<String>) -> Option<String> {
    non_empty_opt(value).map(|raw| {
        if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("data:") {
            raw
        } else if raw.starts_with("//") {
            format!("https:{raw}")
        } else if raw.starts_with('/') {
            format!("{}{raw}", base.trim_end_matches('/'))
        } else {
            format!("{}/{raw}", base.trim_end_matches('/'))
        }
    })
}

/// Normalizes an Xtream server base URL (`http(s)://host[:port]`), forgiving pasted `/player_api.php` or `/get.php` URLs.
pub fn normalize_xtream_base(server_url: &str) -> VesselResult<String> {
    let raw = server_url.trim();
    if raw.is_empty() {
        return Err(VesselError::InvalidInput(
            "Xtream server URL cannot be empty".into(),
        ));
    }
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    let parsed = Url::parse(&with_scheme)
        .map_err(|err| VesselError::InvalidInput(format!("invalid Xtream server URL: {err}")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(VesselError::InvalidInput(format!(
                "unsupported Xtream server URL scheme '{other}'"
            )));
        }
    }
    let Some(host) = parsed.host_str() else {
        return Err(VesselError::InvalidInput(
            "Xtream server URL missing host".into(),
        ));
    };
    let mut base = format!("{}://{host}", parsed.scheme());
    if let Some(port) = parsed.port() {
        base.push_str(&format!(":{port}"));
    }
    let trimmed_path = parsed.path().trim_matches('/');
    if !trimmed_path.is_empty()
        && !trimmed_path.ends_with(".php")
        && !matches!(trimmed_path, "c" | "live" | "movie" | "series")
    {
        base.push('/');
        base.push_str(trimmed_path);
    }
    Ok(base)
}

fn extract_creds_from_url(server_url: &str) -> (Option<String>, Option<String>) {
    let with_scheme = if server_url.trim().contains("://") {
        server_url.trim().to_string()
    } else {
        format!("http://{}", server_url.trim())
    };
    let Ok(parsed) = Url::parse(&with_scheme) else {
        return (None, None);
    };
    let mut u = None;
    let mut p = None;
    for (k, v) in parsed.query_pairs() {
        if k.eq_ignore_ascii_case("username") && !v.trim().is_empty() {
            u = Some(v.into_owned());
        } else if k.eq_ignore_ascii_case("password") && !v.trim().is_empty() {
            p = Some(v.into_owned());
        }
    }
    (u, p)
}

/// Builds the default XMLTV EPG endpoint exposed by Xtream Codes servers.
pub fn xtream_xmltv_url(server_url: &str, username: &str, password: &str) -> VesselResult<String> {
    let base = normalize_xtream_base(server_url)?;
    let mut url = Url::parse(&format!("{base}/xmltv.php"))
        .map_err(|err| VesselError::InvalidInput(format!("invalid Xtream xmltv URL: {err}")))?;
    url.query_pairs_mut()
        .append_pair("username", username)
        .append_pair("password", password);
    Ok(url.into())
}

async fn fetch_xtream_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base: &str,
    username: &str,
    password: &str,
    action: &str,
    user_agent: Option<&str>,
) -> VesselResult<Vec<T>> {
    let mut url = Url::parse(&format!("{base}/player_api.php"))
        .map_err(|err| VesselError::InvalidInput(format!("invalid Xtream API URL: {err}")))?;
    url.query_pairs_mut()
        .append_pair("username", username)
        .append_pair("password", password)
        .append_pair("action", action);

    let effective_ua = user_agent
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_XTREAM_UA);
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(60))
        .header(reqwest::header::USER_AGENT, effective_ua)
        .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
        .send()
        .await
        .map_err(|err| VesselError::Network(format!("Xtream '{action}' request failed: {err}")))?;
    if !resp.status().is_success() {
        return Err(VesselError::Network(format!(
            "Xtream '{action}' returned HTTP {}",
            resp.status()
        )));
    }

    let val: serde_json::Value = resp.json().await.map_err(|err| {
        VesselError::Network(format!("Xtream '{action}' JSON decode failed: {err}"))
    })?;
    match val {
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                if let Ok(parsed) = serde_json::from_value::<T>(item) {
                    out.push(parsed);
                }
            }
            Ok(out)
        }
        serde_json::Value::Object(map) => {
            if let Some(auth) = map
                .get("user_info")
                .and_then(|u| u.get("auth"))
                .and_then(|a| a.as_i64())
                && auth == 0
            {
                return Err(VesselError::InvalidInput(
                    "Xtream server rejected username or password (auth=0)".into(),
                ));
            }
            if let Some(msg) = map.get("message").and_then(|m| m.as_str()) {
                return Err(VesselError::Network(format!("Xtream '{action}': {msg}")));
            }
            Ok(Vec::new())
        }
        _ => Ok(Vec::new()),
    }
}

/// Fetches live categories and live channels from an Xtream Codes compatible endpoint.
pub async fn fetch_xtream_channels(
    client: &reqwest::Client,
    server_url: &str,
    username: &str,
    password: &str,
    user_agent: Option<&str>,
) -> VesselResult<Vec<IptvChannel>> {
    let base = normalize_xtream_base(server_url)?;
    let (url_user, url_pass) = extract_creds_from_url(server_url);
    let eff_user = if username.trim().is_empty() {
        url_user.as_deref().unwrap_or("")
    } else {
        username.trim()
    };
    let eff_pass = if password.trim().is_empty() {
        url_pass.as_deref().unwrap_or("")
    } else {
        password.trim()
    };
    if eff_user.is_empty() {
        return Err(VesselError::InvalidInput(
            "Xtream profile is missing a username".into(),
        ));
    }

    let (categories_res, streams_res) = tokio::join!(
        fetch_xtream_json::<XtreamCategory>(
            client,
            &base,
            eff_user,
            eff_pass,
            "get_live_categories",
            user_agent
        ),
        fetch_xtream_json::<XtreamLiveStream>(
            client,
            &base,
            eff_user,
            eff_pass,
            "get_live_streams",
            user_agent
        ),
    );

    let categories: HashMap<String, String> = categories_res
        .unwrap_or_default()
        .into_iter()
        .map(|cat| (json_id_to_string(&cat.category_id), cat.category_name))
        .collect();
    let streams = streams_res?;

    let mut channels = Vec::with_capacity(streams.len());
    for stream in streams {
        let raw_id = json_id_to_string(&stream.stream_id);
        let stream_id = format!("xtream-{raw_id}");
        let cat_key = stream
            .category_id
            .as_ref()
            .map(json_id_to_string)
            .unwrap_or_default();
        let group_title = categories
            .get(&cat_key)
            .cloned()
            .filter(|g| !g.trim().is_empty())
            .unwrap_or_else(|| "Live TV".to_string());
        // Use HLS `.m3u8` output so WKWebView `<video>` plays natively; player UI also offers `.ts`.
        let stream_url = format!("{base}/live/{eff_user}/{eff_pass}/{raw_id}.m3u8");

        let ch_name = stream
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Channel")
            .to_string();

        channels.push(IptvChannel {
            stream_id,
            name: ch_name,
            group_title,
            logo_url: resolve_icon_url(&base, stream.stream_icon),
            stream_url,
            tvg_id: non_empty_opt(stream.epg_channel_id),
            user_agent: user_agent.and_then(|ua| {
                let t = ua.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }),
        });
    }

    Ok(channels)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::extract::{Query, State};
    use axum::http::HeaderMap;
    use axum::routing::get;
    use axum::{Json, Router};
    use tokio::net::TcpListener;

    use super::{fetch_xtream_channels, xtream_xmltv_url};

    #[derive(Clone, Default)]
    struct MockState {
        last_ua: Arc<Mutex<Option<String>>>,
    }

    async fn mock_player_api(
        State(state): State<MockState>,
        headers: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        if let Some(ua) = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
        {
            *state.last_ua.lock().expect("lock ua") = Some(ua.to_string());
        }
        match params.get("action").map(String::as_str) {
            Some("get_live_categories") => Json(serde_json::json!([
                { "category_id": "10", "category_name": "News" },
                { "category_id": 20, "category_name": "Cinema" }
            ])),
            Some("get_live_streams") => Json(serde_json::json!([
                {
                    "stream_id": 401,
                    "name": "Anthology News 4K",
                    "stream_icon": "https://cdn.anthology.example/logo.png",
                    "epg_channel_id": "news.4k",
                    "category_id": "10"
                },
                {
                    "stream_id": "402",
                    "name": "Anthology Film",
                    "stream_icon": "",
                    "epg_channel_id": "",
                    "category_id": 20
                }
            ])),
            _ => Json(serde_json::json!([])),
        }
    }

    #[tokio::test]
    async fn fetches_xtream_categories_streams_and_applies_custom_user_agent() {
        let state = MockState::default();
        let app = Router::new()
            .route("/player_api.php", get(mock_player_api))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock xtream");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let base = format!("http://{addr}");
        let client = reqwest::Client::new();
        let channels = fetch_xtream_channels(
            &client,
            &base,
            "alice",
            "s3cr3t",
            Some("VLC/3.0.20 LibVLC/3.0.20"),
        )
        .await
        .expect("fetch xtream channels");

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].stream_id, "xtream-401");
        assert_eq!(channels[0].group_title, "News");
        assert_eq!(channels[0].tvg_id.as_deref(), Some("news.4k"));
        assert_eq!(
            channels[0].stream_url,
            format!("{base}/live/alice/s3cr3t/401.m3u8")
        );
        assert_eq!(
            channels[0].user_agent.as_deref(),
            Some("VLC/3.0.20 LibVLC/3.0.20")
        );

        assert_eq!(channels[1].stream_id, "xtream-402");
        assert_eq!(channels[1].group_title, "Cinema");
        assert_eq!(channels[1].logo_url, None);
        assert_eq!(channels[1].tvg_id, None);

        assert_eq!(
            state.last_ua.lock().expect("lock").as_deref(),
            Some("VLC/3.0.20 LibVLC/3.0.20")
        );

        let xmltv = xtream_xmltv_url(&base, "alice", "s3cr3t").expect("xmltv url");
        assert_eq!(
            xmltv,
            format!("{base}/xmltv.php?username=alice&password=s3cr3t")
        );
    }
}
