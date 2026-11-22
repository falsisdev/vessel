//! Local loopback HTTP/HLS proxy server (`127.0.0.1:0`) with stateless HMAC sub-URL signing.

pub mod hls;
pub mod session;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, IF_RANGE, RANGE,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use vessel_domain::{StreamCandidate, StreamFlag, StreamKind, VesselError, VesselResult};

use hls::{decode_signed_seg_url, is_hls_playlist, rewrite_m3u8_playlist};
pub use session::{DEFAULT_SESSION_TTL, ProxySession, ProxySessionStore};

/// Decides whether a resolved stream candidate should pass through the local proxy.
///
/// Plain streams (e.g. `.mp4`, `.mkv`) with no custom headers bypass the proxy unless
/// `always_proxy` is enabled, `RequiresProxy` is set, or the stream is HLS / MPEG-TS.
pub fn should_proxy_stream(candidate: &StreamCandidate, always_proxy: bool) -> bool {
    always_proxy
        || !candidate.headers.is_empty()
        || candidate.flags.contains(&StreamFlag::RequiresProxy)
        || matches!(candidate.kind, StreamKind::Hls | StreamKind::MpegTs)
}

type HmacSha256 = Hmac<Sha256>;

/// Ephemeral 32-byte HMAC-SHA256 signer protecting rewritten proxy sub-URLs against open-relay abuse.
#[derive(Debug, Clone)]
pub struct HmacSigner {
    key: [u8; 32],
}

impl HmacSigner {
    /// Generates a fresh 32-byte secret key using the OS cryptographic RNG.
    pub fn generate() -> VesselResult<Self> {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key)
            .map_err(|err| VesselError::Network(format!("generate proxy hmac key: {err}")))?;
        Ok(Self { key })
    }

    /// Creates a signer from a fixed 32-byte key (useful for deterministic tests).
    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Computes a 64-character lowercase hex HMAC-SHA256 over `token` and `encoded_url`.
    pub fn sign(&self, token: &str, encoded_url: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .expect("32-byte key is always valid for HMAC-SHA256");
        mac.update(token.as_bytes());
        mac.update(b":");
        mac.update(encoded_url.as_bytes());
        let bytes = mac.finalize().into_bytes();
        bytes_to_hex(&bytes)
    }

    /// Performs constant-time HMAC-SHA256 verification for a signed sub-URL parameter.
    pub fn verify(&self, token: &str, encoded_url: &str, signature_hex: &str) -> bool {
        let Some(expected) = decode_hex_32(signature_hex) else {
            return false;
        };
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .expect("32-byte key is always valid for HMAC-SHA256");
        mac.update(token.as_bytes());
        mac.update(b":");
        mac.update(encoded_url.as_bytes());
        mac.verify_slice(&expected).is_ok()
    }
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (idx, chunk) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let s = std::str::from_utf8(chunk).ok()?;
        out[idx] = u8::from_str_radix(s, 16).ok()?;
    }
    Some(out)
}

/// JSON payload returned by `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Deserialize)]
struct SegQuery {
    u: String,
    sig: String,
}

#[derive(Clone)]
struct AppState {
    base_url: String,
    sessions: Arc<ProxySessionStore>,
    signer: Arc<HmacSigner>,
    client: reqwest::Client,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

async fn master_stream_handler(
    Path(token): Path<String>,
    State(state): State<AppState>,
    req_headers: HeaderMap,
) -> Response {
    let Some(session) = state.sessions.get(&token) else {
        return (StatusCode::NOT_FOUND, "unknown or expired proxy token").into_response();
    };
    stream_upstream_target(
        &state,
        &token,
        &session.upstream_url,
        &session.headers,
        &req_headers,
    )
    .await
}

async fn seg_stream_handler(
    Path(token): Path<String>,
    Query(query): Query<SegQuery>,
    State(state): State<AppState>,
    req_headers: HeaderMap,
) -> Response {
    let Some(session) = state.sessions.get(&token) else {
        return (StatusCode::NOT_FOUND, "unknown or expired proxy token").into_response();
    };
    let Some(target_url) = decode_signed_seg_url(&state.signer, &token, &query.u, &query.sig)
    else {
        return (
            StatusCode::FORBIDDEN,
            "invalid or tampered segment signature",
        )
            .into_response();
    };
    stream_upstream_target(&state, &token, &target_url, &session.headers, &req_headers).await
}

async fn stream_upstream_target(
    state: &AppState,
    token: &str,
    target_url: &str,
    session_headers: &BTreeMap<String, String>,
    req_headers: &HeaderMap,
) -> Response {
    let mut upstream_req = state.client.get(target_url);
    for (name, value) in session_headers {
        if let (Ok(header_name), Ok(header_val)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            upstream_req = upstream_req.header(header_name, header_val);
        }
    }

    for forward_key in [RANGE, IF_RANGE] {
        if let Some(val) = req_headers.get(&forward_key) {
            upstream_req = upstream_req.header(forward_key, val.clone());
        }
    }

    let upstream_resp = match upstream_req.send().await {
        Ok(resp) => resp,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("[VesselNetworkError]: upstream request failed: {err}"),
            )
                .into_response();
        }
    };

    let status = upstream_resp.status();
    let final_url = upstream_resp.url().as_str().to_owned();
    let content_type = upstream_resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    if status.is_success() && is_hls_playlist(&final_url, content_type.as_deref()) {
        let playlist_text = match upstream_resp.text().await {
            Ok(text) => text,
            Err(err) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    format!("[VesselNetworkError]: read hls playlist: {err}"),
                )
                    .into_response();
            }
        };

        return match rewrite_m3u8_playlist(
            &playlist_text,
            &final_url,
            &state.base_url,
            token,
            &state.signer,
        ) {
            Ok(rewritten) => (
                status,
                [(CONTENT_TYPE, "application/vnd.apple.mpegurl; charset=utf-8")],
                rewritten,
            )
                .into_response(),
            Err(err) => (StatusCode::BAD_GATEWAY, err.to_string()).into_response(),
        };
    }

    let mut builder = Response::builder().status(status);
    for pass_key in [CONTENT_TYPE, CONTENT_LENGTH, CONTENT_RANGE, ACCEPT_RANGES] {
        if let Some(val) = upstream_resp.headers().get(&pass_key) {
            builder = builder.header(pass_key, val.clone());
        }
    }

    let stream = upstream_resp.bytes_stream();
    match builder.body(Body::from_stream(stream)) {
        Ok(resp) => resp,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Localhost-bound `axum` proxy server with shared `reqwest` (`rustls`) pool and HMAC key.
#[derive(Debug)]
pub struct ProxyServer {
    local_addr: SocketAddr,
    sessions: Arc<ProxySessionStore>,
    signer: Arc<HmacSigner>,
    client: reqwest::Client,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ProxyServer {
    /// Binds to `127.0.0.1:0` and starts serving routes on a background Tokio task.
    pub async fn start() -> VesselResult<Self> {
        let signer = Arc::new(HmacSigner::generate()?);
        let sessions = Arc::new(ProxySessionStore::default());
        let client = reqwest::Client::builder()
            .build()
            .map_err(|err| VesselError::Network(format!("build proxy http client: {err}")))?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| VesselError::Network(format!("bind local proxy listener: {err}")))?;
        let local_addr = listener
            .local_addr()
            .map_err(|err| VesselError::Network(format!("read local proxy address: {err}")))?;

        let state = AppState {
            base_url: format!("http://{local_addr}"),
            sessions: Arc::clone(&sessions),
            signer: Arc::clone(&signer),
            client: client.clone(),
        };

        let router = Router::new()
            .route("/health", get(health_handler))
            .route("/s/{token}/master", get(master_stream_handler))
            .route("/s/{token}/seg", get(seg_stream_handler))
            .with_state(state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        Ok(Self {
            local_addr,
            sessions,
            signer,
            client,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Registers a stream target and returns its full local master proxy URL.
    pub fn register_stream(
        &self,
        upstream_url: impl Into<String>,
        headers: BTreeMap<String, String>,
    ) -> VesselResult<String> {
        let token = self.sessions.register(upstream_url, headers)?;
        Ok(format!("{}/s/{token}/master", self.base_url()))
    }

    /// Applies Smart Proxy routing rules and returns the playback URL for a player backend.
    pub fn resolve_playback_url(
        &self,
        candidate: &StreamCandidate,
        always_proxy: bool,
    ) -> VesselResult<String> {
        if candidate.needs_extractor || candidate.kind == StreamKind::Embed {
            return Err(VesselError::InvalidInput(
                "stream candidate requires extractor resolution before playback".into(),
            ));
        }

        if should_proxy_stream(candidate, always_proxy) {
            self.register_stream(candidate.url.clone(), candidate.headers.clone())
        } else {
            Ok(candidate.url.clone())
        }
    }

    /// Returns the actual `127.0.0.1:<port>` address allocated by the OS.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the base HTTP URL (e.g. `http://127.0.0.1:54321`).
    pub fn base_url(&self) -> String {
        format!("http://{}", self.local_addr)
    }

    /// Borrows the ephemeral session store.
    pub fn sessions(&self) -> &ProxySessionStore {
        &self.sessions
    }

    /// Borrows the ephemeral HMAC signer for sub-URL authentication.
    pub fn signer(&self) -> &HmacSigner {
        &self.signer
    }

    /// Borrows the shared `rustls`-backed upstream HTTP client pool.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Signals the background server task to shut down gracefully.
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ProxyServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::Router;
    use axum::http::header::{ACCEPT_RANGES, CONTENT_RANGE, CONTENT_TYPE, RANGE, REFERER};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tokio::net::TcpListener;

    use vessel_domain::{StreamCandidate, StreamFlag, StreamKind, VesselError};

    use super::{HealthResponse, HmacSigner, ProxyServer, should_proxy_stream};

    async fn spawn_mock_upstream() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock upstream");
        let addr = listener.local_addr().expect("mock addr");

        let app = Router::new()
            .route(
                "/video.mp4",
                get(|headers: HeaderMap| async move {
                    let referer = headers
                        .get(REFERER)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if referer != "https://anthology.example" {
                        return (StatusCode::FORBIDDEN, "missing referer").into_response();
                    }

                    let payload = b"0123456789";
                    if let Some(range_val) = headers.get(RANGE).and_then(|v| v.to_str().ok())
                        && range_val == "bytes=2-5"
                    {
                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [(CONTENT_RANGE, "bytes 2-5/10"), (ACCEPT_RANGES, "bytes")],
                            &payload[2..=5],
                        )
                            .into_response();
                    }

                    (
                        StatusCode::OK,
                        [(ACCEPT_RANGES, "bytes")],
                        payload.as_slice(),
                    )
                        .into_response()
                }),
            )
            .route(
                "/hls/master.m3u8",
                get(|headers: HeaderMap| async move {
                    if headers.get(REFERER).and_then(|v| v.to_str().ok())
                        != Some("https://anthology.example")
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    (
                        [(CONTENT_TYPE, "application/vnd.apple.mpegurl")],
                        "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000000\n720p/index.m3u8\n",
                    )
                        .into_response()
                }),
            )
            .route(
                "/hls/720p/index.m3u8",
                get(|headers: HeaderMap| async move {
                    if headers.get(REFERER).and_then(|v| v.to_str().ok())
                        != Some("https://anthology.example")
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    (
                        [(CONTENT_TYPE, "application/vnd.apple.mpegurl")],
                        "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"../enc.key\"\n#EXTINF:4.0,\nseg0.ts\n",
                    )
                        .into_response()
                }),
            )
            .route(
                "/hls/enc.key",
                get(|headers: HeaderMap| async move {
                    if headers.get(REFERER).and_then(|v| v.to_str().ok())
                        != Some("https://anthology.example")
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    b"0123456789abcdef".as_slice().into_response()
                }),
            )
            .route(
                "/hls/720p/seg0.ts",
                get(|headers: HeaderMap| async move {
                    if headers.get(REFERER).and_then(|v| v.to_str().ok())
                        != Some("https://anthology.example")
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    b"ts-packet-bytes".as_slice().into_response()
                }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn binds_localhost_ephemeral_port_and_serves_health_endpoint() {
        let mut server = ProxyServer::start().await.expect("start local proxy");
        assert!(server.local_addr().ip().is_loopback());
        assert!(server.local_addr().port() > 0);

        let url = format!("{}/health", server.base_url());
        let resp = server
            .client()
            .get(&url)
            .send()
            .await
            .expect("call /health");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        let body: HealthResponse = resp.json().await.expect("parse health json");
        assert_eq!(
            body,
            HealthResponse {
                status: "ok".into()
            }
        );

        server.shutdown();
    }

    #[tokio::test]
    async fn streams_bytes_with_injected_headers_and_range_support() {
        let upstream_base = spawn_mock_upstream().await;
        let server = ProxyServer::start().await.expect("start local proxy");

        // Unknown token must be rejected (no open relay).
        let bad_resp = server
            .client()
            .get(format!("{}/s/invalid-token/master", server.base_url()))
            .send()
            .await
            .expect("call invalid token");
        assert_eq!(bad_resp.status(), reqwest::StatusCode::NOT_FOUND);

        let mut headers = BTreeMap::new();
        headers.insert("Referer".into(), "https://anthology.example".into());
        let proxy_url = server
            .register_stream(format!("{upstream_base}/video.mp4"), headers)
            .expect("register stream");

        let full_resp = server
            .client()
            .get(&proxy_url)
            .send()
            .await
            .expect("stream full");
        assert_eq!(full_resp.status(), reqwest::StatusCode::OK);
        assert_eq!(
            full_resp.bytes().await.expect("read full body").as_ref(),
            b"0123456789"
        );

        let range_resp = server
            .client()
            .get(&proxy_url)
            .header(RANGE, "bytes=2-5")
            .send()
            .await
            .expect("stream range");
        assert_eq!(range_resp.status(), reqwest::StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            range_resp
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|v| v.to_str().ok()),
            Some("bytes 2-5/10")
        );
        assert_eq!(
            range_resp.bytes().await.expect("read range body").as_ref(),
            b"2345"
        );
    }

    #[tokio::test]
    async fn rewrites_nested_hls_playlists_and_enforces_hmac_signatures() {
        let upstream_base = spawn_mock_upstream().await;
        let server = ProxyServer::start().await.expect("start local proxy");

        let mut headers = BTreeMap::new();
        headers.insert("Referer".into(), "https://anthology.example".into());
        let master_proxy_url = server
            .register_stream(format!("{upstream_base}/hls/master.m3u8"), headers)
            .expect("register master playlist");

        let master_body = server
            .client()
            .get(&master_proxy_url)
            .send()
            .await
            .expect("fetch master playlist")
            .text()
            .await
            .expect("read master text");
        let variant_url = master_body
            .lines()
            .find(|line| !line.starts_with('#') && !line.is_empty())
            .expect("rewritten variant URL");

        // Tampered HMAC signature must be rejected with 403 Forbidden.
        let tampered_url = format!("{variant_url}0");
        let tampered_resp = server
            .client()
            .get(&tampered_url)
            .send()
            .await
            .expect("fetch tampered seg");
        assert_eq!(tampered_resp.status(), reqwest::StatusCode::FORBIDDEN);

        // Valid variant playlist URL must recursively rewrite #EXT-X-KEY and segment lines.
        let media_body = server
            .client()
            .get(variant_url)
            .send()
            .await
            .expect("fetch variant playlist")
            .text()
            .await
            .expect("read variant text");

        let key_line = media_body
            .lines()
            .find(|line| line.starts_with("#EXT-X-KEY:"))
            .expect("key line");
        let key_url = key_line
            .split("URI=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("rewritten key URL");
        let seg_url = media_body
            .lines()
            .find(|line| !line.starts_with('#') && !line.is_empty())
            .expect("rewritten seg0 URL");

        let key_bytes = server
            .client()
            .get(key_url)
            .send()
            .await
            .expect("fetch key")
            .bytes()
            .await
            .expect("read key bytes");
        assert_eq!(key_bytes.as_ref(), b"0123456789abcdef");

        let seg_bytes = server
            .client()
            .get(seg_url)
            .send()
            .await
            .expect("fetch seg0")
            .bytes()
            .await
            .expect("read seg0 bytes");
        assert_eq!(seg_bytes.as_ref(), b"ts-packet-bytes");
    }

    #[test]
    fn signs_and_verifies_sub_url_hmac_rejecting_tampering() {
        let signer = HmacSigner::generate().expect("generate hmac key");
        let token = "token-anthology";
        let encoded_url = "aHR0cHM6Ly9jZG4uYW50aG9sb2d5LmV4YW1wbGUvc2VnMS50cw";

        let sig = signer.sign(token, encoded_url);
        assert_eq!(sig.len(), 64);
        assert!(signer.verify(token, encoded_url, &sig));

        assert!(!signer.verify("other-token", encoded_url, &sig));
        assert!(!signer.verify(token, "aHR0cHM6Ly9ldmlsLmV4YW1wbGU", &sig));
        assert!(!signer.verify(token, encoded_url, "deadbeef"));
        assert!(!signer.verify(token, encoded_url, &"0".repeat(64)));

        let fixed = HmacSigner::from_key([7u8; 32]);
        assert_eq!(
            fixed.sign(token, encoded_url),
            fixed.sign(token, encoded_url)
        );
    }

    fn sample_candidate(url: &str, kind: StreamKind) -> StreamCandidate {
        StreamCandidate {
            url: url.into(),
            kind,
            quality: Some("1080p".into()),
            label: "Anthology CDN".into(),
            language: Some("en".into()),
            audio_type: None,
            priority: None,
            is_default: true,
            flags: Vec::new(),
            headers: BTreeMap::new(),
            needs_extractor: false,
            subtitles: Vec::new(),
        }
    }

    #[tokio::test]
    async fn routes_streams_with_smart_proxy_policy() {
        let server = ProxyServer::start().await.expect("start local proxy");

        // Plain MP4 with no headers bypasses the proxy unless always_proxy is enabled.
        let plain_mp4 =
            sample_candidate("https://cdn.anthology.example/movie.mp4", StreamKind::Mp4);
        assert!(!should_proxy_stream(&plain_mp4, false));
        let direct_url = server
            .resolve_playback_url(&plain_mp4, false)
            .expect("resolve plain mp4 directly");
        assert_eq!(direct_url, plain_mp4.url);

        assert!(should_proxy_stream(&plain_mp4, true));
        let forced_proxy_url = server
            .resolve_playback_url(&plain_mp4, true)
            .expect("force proxy plain mp4");
        assert!(forced_proxy_url.starts_with(&format!("{}/s/", server.base_url())));
        assert!(forced_proxy_url.ends_with("/master"));

        // Custom headers force proxying so secrets are never passed to the player.
        let mut headered_mp4 = plain_mp4.clone();
        headered_mp4
            .headers
            .insert("Referer".into(), "https://anthology.example".into());
        assert!(should_proxy_stream(&headered_mp4, false));
        let headered_url = server
            .resolve_playback_url(&headered_mp4, false)
            .expect("proxy headered mp4");
        assert!(headered_url.starts_with(&format!("{}/s/", server.base_url())));

        // RequiresProxy flag forces proxying even without headers.
        let mut flagged_mkv =
            sample_candidate("https://cdn.anthology.example/episode.mkv", StreamKind::Mkv);
        flagged_mkv.flags.push(StreamFlag::RequiresProxy);
        assert!(should_proxy_stream(&flagged_mkv, false));
        let flagged_url = server
            .resolve_playback_url(&flagged_mkv, false)
            .expect("proxy flagged mkv");
        assert!(flagged_url.starts_with(&format!("{}/s/", server.base_url())));

        // HLS & MPEG-TS always go through the proxy (rewriting / reconnect).
        let hls = sample_candidate("https://cdn.anthology.example/master.m3u8", StreamKind::Hls);
        let mpeg_ts =
            sample_candidate("https://iptv.anthology.example/live.ts", StreamKind::MpegTs);
        assert!(should_proxy_stream(&hls, false));
        assert!(should_proxy_stream(&mpeg_ts, false));

        // Embed / unresolved extractor links must be rejected before playback.
        let embed = sample_candidate("https://embed.anthology.example/v/123", StreamKind::Embed);
        let err = server
            .resolve_playback_url(&embed, false)
            .expect_err("reject unresolved embed");
        assert!(matches!(err, VesselError::InvalidInput(_)));

        let mut needs_extract = plain_mp4;
        needs_extract.needs_extractor = true;
        assert!(matches!(
            server.resolve_playback_url(&needs_extract, false),
            Err(VesselError::InvalidInput(_))
        ));
    }
}
