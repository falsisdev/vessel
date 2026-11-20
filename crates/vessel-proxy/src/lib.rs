//! Local loopback HTTP/HLS proxy server (`127.0.0.1:0`) with stateless HMAC sub-URL signing.

pub mod session;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
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
use vessel_domain::{VesselError, VesselResult};

pub use session::{DEFAULT_SESSION_TTL, ProxySession, ProxySessionStore};

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

#[derive(Clone)]
struct AppState {
    sessions: Arc<ProxySessionStore>,
    #[allow(dead_code)]
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

    let mut upstream_req = state.client.get(&session.upstream_url);
    for (name, value) in &session.headers {
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
            sessions: Arc::clone(&sessions),
            signer: Arc::clone(&signer),
            client: client.clone(),
        };

        let router = Router::new()
            .route("/health", get(health_handler))
            .route("/s/{token}/master", get(master_stream_handler))
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
    use axum::http::header::{ACCEPT_RANGES, CONTENT_RANGE, RANGE, REFERER};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tokio::net::TcpListener;

    use super::{HealthResponse, HmacSigner, ProxyServer};

    async fn spawn_mock_upstream() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock upstream");
        let addr = listener.local_addr().expect("mock addr");

        let app = Router::new().route(
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
}
