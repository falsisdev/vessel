//! Local loopback HTTP/HLS proxy server (`127.0.0.1:0`) with stateless HMAC sub-URL signing.

pub mod hls;
pub mod session;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::SeekFrom;
use std::net::SocketAddr;
use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    ACCEPT, ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, CONTENT_LENGTH, CONTENT_RANGE,
    CONTENT_TYPE, IF_RANGE, RANGE, USER_AGENT,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_util::io::ReaderStream;
use vessel_domain::{StreamCandidate, StreamFlag, StreamKind, VesselError, VesselResult};

use hls::{decode_signed_seg_url, is_hls_playlist, rewrite_m3u8_playlist};
pub use session::{DEFAULT_SESSION_TTL, ProxySession, ProxySessionStore};

const DEFAULT_PROXY_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const MAX_SNIFFED_PLAYLIST_BYTES: usize = 8 * 1024 * 1024;

/// Decides whether a resolved stream candidate should pass through the local proxy.
///
/// Plain HTTPS streams (e.g. `.mp4`, `.mkv`) with no custom headers bypass the proxy unless
/// `always_proxy` is enabled, `RequiresProxy` is set, the URL uses cleartext `http://`
/// (which mobile OS policies block directly), or the stream is `HLS` / `MPEG-TS` / `Auto`.
pub fn should_proxy_stream(candidate: &StreamCandidate, always_proxy: bool) -> bool {
    let is_cleartext_http = candidate
        .url
        .trim()
        .to_ascii_lowercase()
        .starts_with("http://");
    always_proxy
        || is_cleartext_http
        || !candidate.headers.is_empty()
        || candidate.flags.contains(&StreamFlag::RequiresProxy)
        || matches!(
            candidate.kind,
            StreamKind::Hls | StreamKind::MpegTs | StreamKind::Auto
        )
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

#[derive(Debug, Default)]
struct LocalDirStore {
    dirs: RwLock<HashMap<String, PathBuf>>,
}

impl LocalDirStore {
    fn register(&self, root: PathBuf) -> VesselResult<String> {
        let canon = std::fs::canonicalize(&root).map_err(|err| {
            VesselError::NotFound(format!("local playback dir '{}': {err}", root.display()))
        })?;
        let mut raw = [0u8; 16];
        getrandom::fill(&mut raw)
            .map_err(|err| VesselError::Network(format!("generate local proxy token: {err}")))?;
        let token = bytes_to_hex(&raw);
        let mut guard = self
            .dirs
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.insert(token.clone(), canon);
        Ok(token)
    }

    fn get(&self, token: &str) -> Option<PathBuf> {
        self.dirs
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(token)
            .cloned()
    }
}

#[derive(Clone)]
struct AppState {
    base_url: String,
    sessions: Arc<ProxySessionStore>,
    local_dirs: Arc<LocalDirStore>,
    signer: Arc<HmacSigner>,
    client: reqwest::Client,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

async fn master_stream_handler(
    method: Method,
    Path(token): Path<String>,
    State(state): State<AppState>,
    req_headers: HeaderMap,
) -> Response {
    let Some(session) = state.sessions.get(&token) else {
        return (StatusCode::NOT_FOUND, "unknown or expired proxy token").into_response();
    };
    stream_upstream_target(
        &state,
        &method,
        &token,
        &session.upstream_url,
        &session.headers,
        &req_headers,
    )
    .await
}

async fn seg_stream_handler(
    method: Method,
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
    stream_upstream_target(
        &state,
        &method,
        &token,
        &target_url,
        &session.headers,
        &req_headers,
    )
    .await
}

fn local_content_type(path: &FsPath) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("m3u8") => "application/vnd.apple.mpegurl; charset=utf-8",
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("m4s") => "video/iso.segment",
        Some("ts") => "video/mp2t",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("aac") => "audio/aac",
        Some("vtt") => "text/vtt; charset=utf-8",
        Some("srt") => "application/x-subrip; charset=utf-8",
        Some("ass") | Some("ssa") => "text/plain; charset=utf-8",
        Some("key") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn parse_byte_range(header_val: &str, file_len: u64) -> Option<Result<(u64, u64), ()>> {
    let spec = header_val.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (start_str, end_str) = spec.split_once('-')?;
    if file_len == 0 {
        return Some(Err(()));
    }
    if start_str.is_empty() {
        let suffix: u64 = end_str.trim().parse().ok()?;
        if suffix == 0 {
            return Some(Err(()));
        }
        let start = file_len.saturating_sub(suffix);
        return Some(Ok((start, file_len - 1)));
    }
    let start: u64 = start_str.trim().parse().ok()?;
    if start >= file_len {
        return Some(Err(()));
    }
    let end = if end_str.trim().is_empty() {
        file_len - 1
    } else {
        end_str.trim().parse::<u64>().ok()?.min(file_len - 1)
    };
    if end < start {
        return Some(Err(()));
    }
    Some(Ok((start, end)))
}

async fn local_file_handler(
    Path((token, rel_path)): Path<(String, String)>,
    State(state): State<AppState>,
    req_headers: HeaderMap,
) -> Response {
    let Some(root_dir) = state.local_dirs.get(&token) else {
        return (StatusCode::NOT_FOUND, "unknown or expired local token").into_response();
    };
    let clean_rel = rel_path.trim_start_matches('/');
    if clean_rel.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty local path").into_response();
    }
    let candidate = FsPath::new(clean_rel);
    if candidate.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return (StatusCode::FORBIDDEN, "path traversal forbidden").into_response();
    }

    let joined = root_dir.join(candidate);
    let canon_target = match tokio::fs::canonicalize(&joined).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::NOT_FOUND, "local media file not found").into_response(),
    };
    if !canon_target.starts_with(&root_dir) {
        return (StatusCode::FORBIDDEN, "path escapes bundle directory").into_response();
    }

    let mut file = match tokio::fs::File::open(&canon_target).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, "local media file not readable").into_response(),
    };
    let Ok(meta) = file.metadata().await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if !meta.is_file() {
        return (StatusCode::NOT_FOUND, "not a regular file").into_response();
    }
    let file_len = meta.len();
    let content_type = local_content_type(&canon_target);

    if let Some(range_val) = req_headers.get(RANGE).and_then(|v| v.to_str().ok())
        && let Some(parsed) = parse_byte_range(range_val, file_len)
    {
        match parsed {
            Ok((start, end)) => {
                let length = end - start + 1;
                if file.seek(SeekFrom::Start(start)).await.is_err() {
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
                let stream = ReaderStream::new(file.take(length));
                return match Response::builder()
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(CONTENT_TYPE, content_type)
                    .header(CONTENT_LENGTH, length.to_string())
                    .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_len}"))
                    .header(ACCEPT_RANGES, "bytes")
                    .body(Body::from_stream(stream))
                {
                    Ok(resp) => resp,
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                };
            }
            Err(()) => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(CONTENT_RANGE, format!("bytes */{file_len}"))
                    .body(Body::empty())
                    .unwrap_or_else(|_| StatusCode::RANGE_NOT_SATISFIABLE.into_response());
            }
        }
    }

    let stream = ReaderStream::new(file);
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_LENGTH, file_len.to_string())
        .header(ACCEPT_RANGES, "bytes")
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn should_sniff_hls_body(final_url: &str, content_type: Option<&str>) -> bool {
    let path_lower = final_url
        .split(['?', '#'])
        .next()
        .unwrap_or(final_url)
        .to_ascii_lowercase();
    for ext in [
        ".mp4", ".m4v", ".mkv", ".webm", ".avi", ".mov", ".ts", ".m4s", ".m4a", ".aac", ".mp3",
        ".key", ".vtt", ".srt", ".ass", ".ssa", ".jpg", ".jpeg", ".png", ".webp",
    ] {
        if path_lower.ends_with(ext) {
            return false;
        }
    }
    match content_type.map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase()) {
        None => true,
        Some(ct) => {
            !ct.starts_with("video/")
                && !ct.starts_with("audio/")
                && !ct.starts_with("image/")
        }
    }
}

fn looks_like_extm3u(bytes: &[u8]) -> bool {
    let stripped = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let trimmed = stripped
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .map_or(&[][..], |idx| &stripped[idx..]);
    trimmed.starts_with(b"#EXTM3U")
}

async fn stream_upstream_target(
    state: &AppState,
    method: &Method,
    token: &str,
    target_url: &str,
    session_headers: &BTreeMap<String, String>,
    req_headers: &HeaderMap,
) -> Response {
    let is_head = *method == Method::HEAD;
    let mut upstream_req = if is_head {
        state.client.head(target_url)
    } else {
        state.client.get(target_url)
    };
    let mut has_ua = false;
    let mut has_accept = false;
    for (name, value) in session_headers {
        if name.eq_ignore_ascii_case("user-agent") {
            has_ua = true;
        } else if name.eq_ignore_ascii_case("accept") {
            has_accept = true;
        }
        if let (Ok(header_name), Ok(header_val)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            upstream_req = upstream_req.header(header_name, header_val);
        }
    }
    if !has_ua {
        upstream_req = upstream_req.header(USER_AGENT, DEFAULT_PROXY_USER_AGENT);
    }
    if !has_accept {
        upstream_req = upstream_req.header(ACCEPT, "*/*");
    }

    for forward_key in [RANGE, IF_RANGE] {
        if let Some(val) = req_headers.get(&forward_key) {
            upstream_req = upstream_req.header(forward_key, val.clone());
        }
    }

    let mut upstream_resp = match upstream_req.send().await {
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

    if !is_head && status.is_success() && is_hls_playlist(&final_url, content_type.as_deref()) {
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

    if !is_head && status.is_success() && should_sniff_hls_body(&final_url, content_type.as_deref()) {
        let first_chunk = match upstream_resp.chunk().await {
            Ok(c) => c,
            Err(err) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    format!("[VesselNetworkError]: read upstream chunk: {err}"),
                )
                    .into_response();
            }
        };
        let Some(first) = first_chunk else {
            return Response::builder()
                .status(status)
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        };
        if looks_like_extm3u(&first) {
            let mut buf = first.to_vec();
            while buf.len() < MAX_SNIFFED_PLAYLIST_BYTES {
                match upstream_resp.chunk().await {
                    Ok(Some(chunk)) => {
                        if buf.len() + chunk.len() > MAX_SNIFFED_PLAYLIST_BYTES {
                            return (
                                StatusCode::BAD_GATEWAY,
                                "[VesselNetworkError]: sniffed HLS playlist exceeded size limit",
                            )
                                .into_response();
                        }
                        buf.extend_from_slice(&chunk);
                    }
                    Ok(None) => break,
                    Err(err) => {
                        return (
                            StatusCode::BAD_GATEWAY,
                            format!("[VesselNetworkError]: read sniffed hls playlist: {err}"),
                        )
                            .into_response();
                    }
                }
            }
            let playlist_text = String::from_utf8_lossy(&buf);
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
        let (client_read, mut server_write) = tokio::io::duplex(64 * 1024);
        tokio::spawn(async move {
            if server_write.write_all(&first).await.is_err() {
                return;
            }
            while let Ok(Some(chunk)) = upstream_resp.chunk().await {
                if server_write.write_all(&chunk).await.is_err() {
                    break;
                }
            }
            let _ = server_write.shutdown().await;
        });
        return match builder.body(Body::from_stream(ReaderStream::new(client_read))) {
            Ok(resp) => resp,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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

async fn add_cors_headers(mut resp: Response) -> Response {
    let headers = resp.headers_mut();
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, HEAD, OPTIONS"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Range, If-Range, Content-Type"),
    );
    headers.insert(
        ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static("Content-Length, Content-Range, Accept-Ranges, Content-Type"),
    );
    resp
}

/// Localhost-bound `axum` proxy server with shared `reqwest` (`rustls`) pool and HMAC key.
#[derive(Debug)]
pub struct ProxyServer {
    local_addr: SocketAddr,
    sessions: Arc<ProxySessionStore>,
    local_dirs: Arc<LocalDirStore>,
    signer: Arc<HmacSigner>,
    client: reqwest::Client,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ProxyServer {
    /// Binds to `127.0.0.1:0` and starts serving routes on a background Tokio task.
    pub async fn start() -> VesselResult<Self> {
        let signer = Arc::new(HmacSigner::generate()?);
        let sessions = Arc::new(ProxySessionStore::default());
        let local_dirs = Arc::new(LocalDirStore::default());
        let client = vessel_domain::http_client_builder()
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
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
            local_dirs: Arc::clone(&local_dirs),
            signer: Arc::clone(&signer),
            client: client.clone(),
        };

        let router = Router::new()
            .route("/health", get(health_handler))
            .route("/s/{token}/master", get(master_stream_handler))
            .route("/s/{token}/seg", get(seg_stream_handler))
            .route("/local/{token}/{*path}", get(local_file_handler))
            .with_state(state)
            .layer(middleware::map_response(add_cors_headers));
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
            local_dirs,
            signer,
            client,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Registers a local bundle directory and returns its base URL prefix (`http://127.0.0.1:<port>/local/<token>`).
    pub fn register_local_dir(&self, root_dir: impl Into<PathBuf>) -> VesselResult<String> {
        let token = self.local_dirs.register(root_dir.into())?;
        Ok(format!("{}/local/{token}", self.base_url()))
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

        if candidate.url.starts_with(&self.base_url()) {
            return Ok(candidate.url.clone());
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
            skip_segments: Vec::new(),
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

        // HLS, MPEG-TS, Auto, and cleartext HTTP always go through the proxy (rewriting / Android cleartext safety).
        let hls = sample_candidate("https://cdn.anthology.example/master.m3u8", StreamKind::Hls);
        let mpeg_ts =
            sample_candidate("https://iptv.anthology.example/live.ts", StreamKind::MpegTs);
        let auto_stream = sample_candidate("https://cdn.anthology.example/live/1", StreamKind::Auto);
        let cleartext_mp4 =
            sample_candidate("http://cdn.anthology.example/movie.mp4", StreamKind::Mp4);
        assert!(should_proxy_stream(&hls, false));
        assert!(should_proxy_stream(&mpeg_ts, false));
        assert!(should_proxy_stream(&auto_stream, false));
        assert!(should_proxy_stream(&cleartext_mp4, false));

        // Already-local proxy URLs targeting this proxy server must never be double-proxied.
        let loopback =
            sample_candidate(&format!("{}/s/abc/master", server.base_url()), StreamKind::Auto);
        assert_eq!(
            server
                .resolve_playback_url(&loopback, true)
                .expect("preserve own proxy url"),
            loopback.url
        );

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

    #[tokio::test]
    async fn serves_local_offline_files_with_range_and_traversal_guard() {
        let server = ProxyServer::start().await.expect("start proxy");
        let dir = std::env::temp_dir().join(format!("vessel-proxy-local-{}", std::process::id()));
        let sub_dir = dir.join("audio_0");
        tokio::fs::create_dir_all(&sub_dir).await.expect("mkdir");
        let video_path = dir.join("video.mp4");
        let seg_path = sub_dir.join("seg.ts");
        tokio::fs::write(&video_path, b"0123456789ABCDEF")
            .await
            .expect("write video");
        tokio::fs::write(&seg_path, b"AUDIO_SEG")
            .await
            .expect("write audio seg");

        let base = server.register_local_dir(&dir).expect("register dir");
        let client = reqwest::Client::new();

        // Range request returns 206 Partial Content with exact slice.
        let resp = client
            .get(format!("{base}/video.mp4"))
            .header(RANGE, "bytes=4-9")
            .send()
            .await
            .expect("range fetch");
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(resp.bytes().await.expect("bytes").as_ref(), b"456789");

        // Nested relative subpath resolves cleanly.
        let nested = client
            .get(format!("{base}/audio_0/seg.ts"))
            .send()
            .await
            .expect("nested fetch");
        assert_eq!(nested.status(), StatusCode::OK);
        assert_eq!(nested.bytes().await.expect("bytes").as_ref(), b"AUDIO_SEG");

        // Parent directory traversal is blocked.
        let escaped = client
            .get(format!("{base}/../etc/passwd"))
            .send()
            .await
            .expect("traversal request");
        assert!(matches!(
            escaped.status(),
            StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn sniffs_mislabeled_hls_playlists_and_rewrites_segments() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new().route(
            "/live/channel",
            get(|| async {
                (
                    [(CONTENT_TYPE, "text/plain; charset=utf-8")],
                    "#EXTM3U\n#EXTINF:4.0,\nchunk_0.ts\n",
                )
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let server = ProxyServer::start().await.expect("start proxy");
        let proxy_url = server
            .register_stream(format!("http://{addr}/live/channel"), BTreeMap::new())
            .expect("register");
        let resp = server.client().get(&proxy_url).send().await.expect("fetch");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/vnd.apple.mpegurl; charset=utf-8")
        );
        let body = resp.text().await.expect("body");
        assert!(body.contains("/seg?u="));
        assert!(!body.contains("\nchunk_0.ts"));
    }
}
