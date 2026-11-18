//! Local loopback HTTP/HLS proxy server (`127.0.0.1:0`) with stateless HMAC sub-URL signing.

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use vessel_domain::{VesselError, VesselResult};

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

fn bytes_to_hex(bytes: &[u8]) -> String {
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

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

/// Localhost-bound `axum` proxy server with shared `reqwest` (`rustls`) pool and HMAC key.
#[derive(Debug)]
pub struct ProxyServer {
    local_addr: SocketAddr,
    signer: Arc<HmacSigner>,
    client: reqwest::Client,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ProxyServer {
    /// Binds to `127.0.0.1:0` and starts serving routes on a background Tokio task.
    pub async fn start() -> VesselResult<Self> {
        let signer = Arc::new(HmacSigner::generate()?);
        let client = reqwest::Client::builder()
            .build()
            .map_err(|err| VesselError::Network(format!("build proxy http client: {err}")))?;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| VesselError::Network(format!("bind local proxy listener: {err}")))?;
        let local_addr = listener
            .local_addr()
            .map_err(|err| VesselError::Network(format!("read local proxy address: {err}")))?;

        let router = Router::new().route("/health", get(health_handler));
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
            signer,
            client,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Returns the actual `127.0.0.1:<port>` address allocated by the OS.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the base HTTP URL (e.g. `http://127.0.0.1:54321`).
    pub fn base_url(&self) -> String {
        format!("http://{}", self.local_addr)
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
    use super::{HealthResponse, HmacSigner, ProxyServer};

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
