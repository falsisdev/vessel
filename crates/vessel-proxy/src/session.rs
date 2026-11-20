//! Ephemeral token-based proxy session store preventing open-relay abuse.

use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use vessel_domain::{VesselError, VesselResult};

use crate::bytes_to_hex;

/// Default time-to-live for registered proxy stream tokens.
pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// Upstream target and injected headers bound to a single stream token.
#[derive(Debug, Clone)]
pub struct ProxySession {
    pub upstream_url: String,
    pub headers: BTreeMap<String, String>,
    created_at: Instant,
}

/// Thread-safe in-memory registry mapping ephemeral random tokens to upstream stream targets.
#[derive(Debug)]
pub struct ProxySessionStore {
    ttl: Duration,
    sessions: RwLock<HashMap<String, ProxySession>>,
}

impl Default for ProxySessionStore {
    fn default() -> Self {
        Self::new(DEFAULT_SESSION_TTL)
    }
}

impl ProxySessionStore {
    /// Creates a new session store with the given token TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an upstream HTTP(S) URL + headers and returns a random 32-char hex token.
    pub fn register(
        &self,
        upstream_url: impl Into<String>,
        headers: BTreeMap<String, String>,
    ) -> VesselResult<String> {
        let upstream_url = upstream_url.into().trim().to_owned();
        if !(upstream_url.starts_with("http://") || upstream_url.starts_with("https://")) {
            return Err(VesselError::InvalidInput(
                "proxy upstream url must use http:// or https://".into(),
            ));
        }

        let mut token_bytes = [0u8; 16];
        getrandom::fill(&mut token_bytes)
            .map_err(|err| VesselError::Network(format!("generate proxy session token: {err}")))?;
        let token = bytes_to_hex(&token_bytes);

        let mut guard = self
            .sessions
            .write()
            .expect("proxy session store write lock poisoned");
        let ttl = self.ttl;
        guard.retain(|_, session| session.created_at.elapsed() <= ttl);
        guard.insert(
            token.clone(),
            ProxySession {
                upstream_url,
                headers,
                created_at: Instant::now(),
            },
        );
        Ok(token)
    }

    /// Looks up a non-expired session by token.
    pub fn get(&self, token: &str) -> Option<ProxySession> {
        let guard = self
            .sessions
            .read()
            .expect("proxy session store read lock poisoned");
        let session = guard.get(token)?;
        if session.created_at.elapsed() > self.ttl {
            return None;
        }
        Some(session.clone())
    }

    /// Revokes an active token immediately.
    pub fn revoke(&self, token: &str) -> bool {
        self.sessions
            .write()
            .expect("proxy session store write lock poisoned")
            .remove(token)
            .is_some()
    }
}
