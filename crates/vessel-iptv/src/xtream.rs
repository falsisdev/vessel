//! Xtream Codes HTTP client (`player_api.php`) for live categories and streams.

use std::collections::HashMap;

use serde::Deserialize;
use url::Url;
use vessel_domain::{VesselError, VesselResult};

use crate::IptvChannel;

#[derive(Debug, Deserialize)]
struct XtreamCategory {
    category_id: serde_json::Value,
    category_name: String,
}

#[derive(Debug, Deserialize)]
struct XtreamLiveStream {
    stream_id: serde_json::Value,
    name: String,
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

/// Normalizes an Xtream server base URL (validates HTTP/HTTPS and strips trailing slashes).
pub fn normalize_xtream_base(server_url: &str) -> VesselResult<String> {
    let trimmed = server_url.trim().trim_end_matches('/');
    let parsed = Url::parse(trimmed)
        .map_err(|err| VesselError::InvalidInput(format!("invalid Xtream server URL: {err}")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(trimmed.to_string()),
        other => Err(VesselError::InvalidInput(format!(
            "unsupported Xtream server URL scheme '{other}'"
        ))),
    }
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
) -> VesselResult<T> {
    let mut url = Url::parse(&format!("{base}/player_api.php"))
        .map_err(|err| VesselError::InvalidInput(format!("invalid Xtream API URL: {err}")))?;
    url.query_pairs_mut()
        .append_pair("username", username)
        .append_pair("password", password)
        .append_pair("action", action);

    let mut req = client.get(url);
    if let Some(ua) = user_agent
        && !ua.trim().is_empty()
    {
        req = req.header(reqwest::header::USER_AGENT, ua.trim());
    }

    let resp = req
        .send()
        .await
        .map_err(|err| VesselError::Network(format!("Xtream '{action}' request failed: {err}")))?;
    if !resp.status().is_success() {
        return Err(VesselError::Network(format!(
            "Xtream '{action}' returned HTTP {}",
            resp.status()
        )));
    }

    resp.json::<T>()
        .await
        .map_err(|err| VesselError::Network(format!("Xtream '{action}' JSON decode failed: {err}")))
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

    let (categories_res, streams_res) = tokio::join!(
        fetch_xtream_json::<Vec<XtreamCategory>>(
            client,
            &base,
            username,
            password,
            "get_live_categories",
            user_agent
        ),
        fetch_xtream_json::<Vec<XtreamLiveStream>>(
            client,
            &base,
            username,
            password,
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
            .unwrap_or_else(|| "Live TV".to_string());
        let stream_url = format!("{base}/live/{username}/{password}/{raw_id}.ts");

        channels.push(IptvChannel {
            stream_id,
            name: stream.name.trim().to_string(),
            group_title,
            logo_url: non_empty_opt(stream.stream_icon),
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
            format!("{base}/live/alice/s3cr3t/401.ts")
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
