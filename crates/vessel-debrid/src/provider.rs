//! Shared HTTP, form encoding & secret helpers for built-in Debrid providers.

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use vessel_domain::{SecretStore, VesselError, VesselResult, http_client_builder};

pub fn build_debrid_http_client() -> VesselResult<Client> {
    http_client_builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Vessel/0.3")
        .build()
        .map_err(|err| VesselError::Network(format!("build debrid HTTP client: {err}")))
}

pub async fn require_secret(secrets: &Arc<dyn SecretStore>, key: &str, label: &str) -> VesselResult<String> {
    let val = secrets
        .get(key)
        .await?
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| VesselError::InvalidInput(format!("{label} API key is not configured")))?;
    Ok(val)
}

pub async fn read_optional_secret(secrets: &Arc<dyn SecretStore>, key: &str) -> Option<String> {
    secrets
        .get(key)
        .await
        .ok()
        .flatten()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Encodes key-value pairs as `application/x-www-form-urlencoded` without requiring extra reqwest features.
pub fn encode_form<K: AsRef<str>, V: AsRef<str>>(pairs: &[(K, V)]) -> String {
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        ser.append_pair(k.as_ref(), v.as_ref());
    }
    ser.finish()
}

pub fn parse_json_response<T: DeserializeOwned>(provider: &str, context: &str, body: &str) -> VesselResult<T> {
    serde_json::from_str(body)
        .map_err(|e| VesselError::Network(format!("{provider} {context} JSON parse error: {e}")))
}

pub fn map_http_err(provider: &str, err: reqwest::Error) -> VesselError {
    if err.is_timeout() {
        VesselError::Timeout
    } else {
        VesselError::Network(format!("{provider} request failed: {err}"))
    }
}

pub fn map_status_err(provider: &str, status: reqwest::StatusCode, body: &str) -> VesselError {
    let snippet: String = body.trim().chars().take(240).collect();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        VesselError::InvalidInput(format!(
            "{provider} authentication failed (HTTP {status}): {snippet}"
        ))
    } else if status.as_u16() == 429 {
        VesselError::Network(format!("{provider} rate limit exceeded (HTTP 429): {snippet}"))
    } else {
        VesselError::Network(format!("{provider} API error (HTTP {status}): {snippet}"))
    }
}
