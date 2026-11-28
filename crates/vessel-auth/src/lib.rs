//! Shared OAuth 2.0 (PKCE Loopback, Android App Link, Device Code) and token refresh manager.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::Query;
use axum::response::Html;
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use url::Url;
use vessel_domain::{
    OAuthClientConfig, OAuthDevicePrompt, OAuthFlowKind, OAuthProviderKind, OAuthStartResult,
    OAuthTokenBundle, SecretStore, VesselError, VesselResult, http_client_builder,
};

/// Default Android App Link redirect URI registered in `AndroidManifest.xml`.
pub const ANDROID_OAUTH_REDIRECT_PREFIX: &str = "vessel://oauth";

/// Static OAuth 2.0 endpoint specification for a supported provider.
#[derive(Debug, Clone)]
pub struct OAuthProviderSpec {
    pub kind: OAuthProviderKind,
    pub authorize_url: String,
    pub token_url: String,
    pub device_code_url: Option<String>,
    pub device_token_url: Option<String>,
    pub scopes: String,
    pub supports_pkce: bool,
    pub plain_pkce_only: bool,
    pub supports_device_code: bool,
}

/// Returns canonical OAuth endpoints for `kind`.
pub fn default_provider_spec(kind: OAuthProviderKind) -> OAuthProviderSpec {
    match kind {
        OAuthProviderKind::GoogleDrive => OAuthProviderSpec {
            kind,
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            device_code_url: Some("https://oauth2.googleapis.com/device/code".into()),
            device_token_url: Some("https://oauth2.googleapis.com/token".into()),
            scopes: "https://www.googleapis.com/auth/drive.appdata https://www.googleapis.com/auth/userinfo.email".into(),
            supports_pkce: true,
            plain_pkce_only: false,
            supports_device_code: true,
        },
        OAuthProviderKind::Trakt => OAuthProviderSpec {
            kind,
            authorize_url: "https://trakt.tv/oauth/authorize".into(),
            token_url: "https://api.trakt.tv/oauth/token".into(),
            device_code_url: Some("https://api.trakt.tv/oauth/device/code".into()),
            device_token_url: Some("https://api.trakt.tv/oauth/device/token".into()),
            scopes: String::new(),
            supports_pkce: true,
            plain_pkce_only: false,
            supports_device_code: true,
        },
        OAuthProviderKind::Simkl => OAuthProviderSpec {
            kind,
            authorize_url: "https://simkl.com/oauth/authorize".into(),
            token_url: "https://api.simkl.com/oauth/token".into(),
            device_code_url: Some("https://api.simkl.com/oauth/pin".into()),
            device_token_url: None,
            scopes: String::new(),
            supports_pkce: false,
            plain_pkce_only: false,
            supports_device_code: true,
        },
        OAuthProviderKind::Anilist => OAuthProviderSpec {
            kind,
            authorize_url: "https://anilist.co/api/v2/oauth/authorize".into(),
            token_url: "https://anilist.co/api/v2/oauth/token".into(),
            device_code_url: None,
            device_token_url: None,
            scopes: String::new(),
            supports_pkce: false,
            plain_pkce_only: false,
            supports_device_code: false,
        },
        OAuthProviderKind::MyAnimeList => OAuthProviderSpec {
            kind,
            authorize_url: "https://myanimelist.net/v1/oauth2/authorize".into(),
            token_url: "https://myanimelist.net/v1/oauth2/token".into(),
            device_code_url: None,
            device_token_url: None,
            scopes: String::new(),
            supports_pkce: true,
            plain_pkce_only: true,
            supports_device_code: false,
        },
        OAuthProviderKind::Kitsu => OAuthProviderSpec {
            kind,
            authorize_url: "https://kitsu.io/api/oauth/authorize".into(),
            token_url: "https://kitsu.io/api/oauth/token".into(),
            device_code_url: None,
            device_token_url: None,
            scopes: String::new(),
            supports_pkce: false,
            plain_pkce_only: false,
            supports_device_code: false,
        },
    }
}

#[derive(Debug, Clone)]
struct PendingAuthFlow {
    provider: OAuthProviderKind,
    code_verifier: Option<String>,
    redirect_uri: String,
}

/// Manages OAuth 2.0 flows, `SecretStore` persistence, and single-flight token refresh.
pub struct OAuthManager {
    secrets: Arc<dyn SecretStore>,
    http: reqwest::Client,
    specs: HashMap<OAuthProviderKind, OAuthProviderSpec>,
    pending: Arc<Mutex<HashMap<String, PendingAuthFlow>>>,
    refresh_locks: Arc<Mutex<HashMap<OAuthProviderKind, Arc<Mutex<()>>>>>,
}

impl std::fmt::Debug for OAuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthManager").finish_non_exhaustive()
    }
}

impl OAuthManager {
    /// Creates an `OAuthManager` with canonical upstream OAuth endpoints.
    pub fn new(secrets: Arc<dyn SecretStore>) -> VesselResult<Self> {
        let http = http_client_builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|err| VesselError::Network(format!("build oauth http client: {err}")))?;
        let mut specs = HashMap::new();
        for kind in [
            OAuthProviderKind::GoogleDrive,
            OAuthProviderKind::Trakt,
            OAuthProviderKind::Simkl,
            OAuthProviderKind::Anilist,
            OAuthProviderKind::MyAnimeList,
            OAuthProviderKind::Kitsu,
        ] {
            specs.insert(kind, default_provider_spec(kind));
        }
        Ok(Self {
            secrets,
            http,
            specs,
            pending: Arc::new(Mutex::new(HashMap::new())),
            refresh_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Overrides endpoint specs (useful for unit tests with local mock HTTP servers).
    pub fn with_custom_spec(mut self, spec: OAuthProviderSpec) -> Self {
        self.specs.insert(spec.kind, spec);
        self
    }

    /// Saves OAuth client credentials (`client_id` / optional `client_secret`) into `SecretStore`.
    pub async fn save_client_config(
        &self,
        provider: OAuthProviderKind,
        config: OAuthClientConfig,
    ) -> VesselResult<()> {
        let client_id = config.client_id.trim().to_owned();
        if client_id.is_empty() {
            return Err(VesselError::InvalidInput(
                "OAuth clientId cannot be empty".into(),
            ));
        }
        let normalized = OAuthClientConfig {
            client_id,
            client_secret: config
                .client_secret
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
        };
        let raw = serde_json::to_string(&normalized)
            .map_err(|err| VesselError::Storage(format!("encode oauth client config: {err}")))?;
        self.secrets
            .set(&provider.client_config_secret_key(), &raw)
            .await
    }

    /// Reads OAuth client credentials from `SecretStore`.
    pub async fn get_client_config(
        &self,
        provider: OAuthProviderKind,
    ) -> VesselResult<Option<OAuthClientConfig>> {
        let Some(raw) = self
            .secrets
            .get(&provider.client_config_secret_key())
            .await?
        else {
            return Ok(None);
        };
        let cfg = serde_json::from_str(&raw)
            .map_err(|err| VesselError::Storage(format!("decode oauth client config: {err}")))?;
        Ok(Some(cfg))
    }

    /// Persists an `OAuthTokenBundle` inside `SecretStore`.
    pub async fn save_token_bundle(
        &self,
        provider: OAuthProviderKind,
        bundle: &OAuthTokenBundle,
    ) -> VesselResult<()> {
        let raw = serde_json::to_string(bundle)
            .map_err(|err| VesselError::Storage(format!("encode oauth token bundle: {err}")))?;
        self.secrets.set(&provider.token_secret_key(), &raw).await
    }

    /// Reads the stored `OAuthTokenBundle` for `provider` without refreshing.
    pub async fn get_token_bundle(
        &self,
        provider: OAuthProviderKind,
    ) -> VesselResult<Option<OAuthTokenBundle>> {
        let Some(raw) = self.secrets.get(&provider.token_secret_key()).await? else {
            return Ok(None);
        };
        let bundle = serde_json::from_str(&raw)
            .map_err(|err| VesselError::Storage(format!("decode oauth token bundle: {err}")))?;
        Ok(Some(bundle))
    }

    /// Disconnects `provider` by deleting its stored tokens from `SecretStore`.
    pub async fn disconnect(&self, provider: OAuthProviderKind) -> VesselResult<()> {
        self.secrets.delete(&provider.token_secret_key()).await
    }

    /// Returns a valid access token, automatically performing a single-flight refresh if near expiry.
    pub async fn get_valid_access_token(&self, provider: OAuthProviderKind) -> VesselResult<String> {
        let bundle = self.get_token_bundle(provider).await?.ok_or_else(|| {
            VesselError::NotFound(format!("{} is not authenticated", provider.as_str()))
        })?;
        if !bundle.is_expiring_soon(now_epoch_secs(), 120) {
            return Ok(bundle.access_token);
        }

        let lock = self.provider_refresh_lock(provider).await;
        let _guard = lock.lock().await;

        // Re-check inside lock so concurrent callers reuse the freshly refreshed token.
        let latest = self.get_token_bundle(provider).await?.ok_or_else(|| {
            VesselError::NotFound(format!("{} is not authenticated", provider.as_str()))
        })?;
        if !latest.is_expiring_soon(now_epoch_secs(), 120) {
            return Ok(latest.access_token);
        }

        let Some(refresh_token) = latest.refresh_token.clone() else {
            return Err(VesselError::InvalidInput(format!(
                "{} access token expired and no refresh_token is available",
                provider.as_str()
            )));
        };

        let refreshed = self.refresh_with_token(provider, &refresh_token).await?;
        let access = refreshed.access_token.clone();
        self.save_token_bundle(provider, &refreshed).await?;
        Ok(access)
    }

    /// Prepares an authorization URL for Desktop Loopback or Android App Link PKCE flows.
    pub async fn start_browser_flow(
        &self,
        provider: OAuthProviderKind,
        flow: OAuthFlowKind,
        redirect_uri: impl Into<String>,
    ) -> VesselResult<OAuthStartResult> {
        if flow == OAuthFlowKind::DeviceCode {
            let prompt = self.start_device_code_flow(provider).await?;
            return Ok(OAuthStartResult::DevicePrompt(prompt));
        }

        let cfg = self.require_client_config(provider).await?;
        let spec = self.spec(provider)?;
        let redirect_uri = redirect_uri.into();
        let state = random_urlsafe(24)?;

        let mut url = Url::parse(&spec.authorize_url)
            .map_err(|err| VesselError::InvalidInput(format!("invalid authorize_url: {err}")))?;

        let (code_verifier, code_challenge, challenge_method) = if spec.supports_pkce {
            let verifier = random_urlsafe(48)?;
            if spec.plain_pkce_only {
                (Some(verifier.clone()), Some(verifier), Some("plain"))
            } else {
                let challenge = pkce_s256(&verifier);
                (Some(verifier), Some(challenge), Some("S256"))
            }
        } else {
            (None, None, None)
        };

        {
            let mut q = url.query_pairs_mut();
            q.append_pair("response_type", "code");
            q.append_pair("client_id", &cfg.client_id);
            q.append_pair("redirect_uri", &redirect_uri);
            q.append_pair("state", &state);
            if !spec.scopes.is_empty() {
                q.append_pair("scope", &spec.scopes);
            }
            if provider == OAuthProviderKind::GoogleDrive {
                q.append_pair("access_type", "offline");
                q.append_pair("prompt", "consent");
            }
            if let (Some(challenge), Some(method)) = (code_challenge, challenge_method) {
                q.append_pair("code_challenge", &challenge);
                q.append_pair("code_challenge_method", method);
            }
        }

        self.pending.lock().await.insert(
            state.clone(),
            PendingAuthFlow {
                provider,
                code_verifier,
                redirect_uri: redirect_uri.clone(),
            },
        );

        Ok(OAuthStartResult::BrowserUrl {
            provider,
            auth_url: url.to_string(),
            state,
            redirect_uri,
        })
    }

    /// Starts a temporary `127.0.0.1:0` loopback HTTP server for desktop browser OAuth + PKCE,
    /// returning the browser prompt and a oneshot receiver that resolves when the redirect completes.
    pub async fn start_loopback_pkce_session(
        self: &Arc<Self>,
        provider: OAuthProviderKind,
    ) -> VesselResult<(OAuthStartResult, oneshot::Receiver<VesselResult<OAuthTokenBundle>>)> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| VesselError::Network(format!("bind oauth loopback port: {err}")))?;
        let port = listener
            .local_addr()
            .map_err(|err| VesselError::Network(format!("read oauth loopback addr: {err}")))?
            .port();
        let redirect_uri = format!("http://127.0.0.1:{port}/callback");
        let start = self
            .start_browser_flow(provider, OAuthFlowKind::LoopbackPkce, redirect_uri)
            .await?;

        let (tx, rx) = oneshot::channel();
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            #[derive(Deserialize)]
            struct CallbackQuery {
                code: Option<String>,
                state: Option<String>,
                error: Option<String>,
                error_description: Option<String>,
            }

            let (done_tx, done_rx) = oneshot::channel::<()>();
            let done_tx = Arc::new(Mutex::new(Some(done_tx)));
            let tx_slot = Arc::new(Mutex::new(Some(tx)));
            let mgr_route = Arc::clone(&manager);

            let app = Router::new().route(
                "/callback",
                get(move |Query(q): Query<CallbackQuery>| {
                    let mgr = Arc::clone(&mgr_route);
                    let tx_slot = Arc::clone(&tx_slot);
                    let done_tx = Arc::clone(&done_tx);
                    async move {
                        let outcome = if let Some(err) = q.error {
                            let desc = q.error_description.unwrap_or_default();
                            Err(VesselError::Network(format!("oauth callback error: {err} {desc}")))
                        } else if let (Some(code), Some(state)) = (q.code, q.state) {
                            mgr.complete_code_callback(&state, &code).await
                        } else {
                            Err(VesselError::InvalidInput(
                                "oauth callback missing code or state".into(),
                            ))
                        };

                        let html = match &outcome {
                            Ok(_) => "<html><body style='font-family:sans-serif;background:#11111b;color:#cdd6f4;text-align:center;padding:48px'><h2>Connected to Vessel</h2><p>Authorization complete. You can close this window and return to Vessel.</p></body></html>".to_owned(),
                            Err(err) => format!(
                                "<html><body style='font-family:sans-serif;background:#11111b;color:#f38ba8;text-align:center;padding:48px'><h2>Vessel Authorization Failed</h2><p>{err}</p></body></html>"
                            ),
                        };

                        if let Some(sender) = tx_slot.lock().await.take() {
                            let _ = sender.send(outcome);
                        }
                        if let Some(done) = done_tx.lock().await.take() {
                            let _ = done.send(());
                        }
                        Html(html)
                    }
                }),
            );

            let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(300), done_rx).await;
            });
            let _ = server.await;
        });

        Ok((start, rx))
    }

    /// Exchanges an authorization `code` + `state` (from Desktop Loopback or Android `vessel://oauth/...`)
    /// for tokens and stores them in `SecretStore`.
    pub async fn complete_code_callback(
        &self,
        state: &str,
        code: &str,
    ) -> VesselResult<OAuthTokenBundle> {
        let pending = self
            .pending
            .lock()
            .await
            .remove(state)
            .ok_or_else(|| VesselError::InvalidInput("unknown or expired oauth state".into()))?;

        let cfg = self.require_client_config(pending.provider).await?;
        let spec = self.spec(pending.provider)?;

        let mut form: Vec<(&str, String)> = vec![
            ("grant_type", "authorization_code".into()),
            ("client_id", cfg.client_id),
            ("code", code.trim().to_owned()),
            ("redirect_uri", pending.redirect_uri),
        ];
        if let Some(secret) = cfg.client_secret {
            form.push(("client_secret", secret));
        }
        if let Some(verifier) = pending.code_verifier {
            form.push(("code_verifier", verifier));
        }

        let bundle = self.exchange_token_form(&spec.token_url, &form).await?;
        self.save_token_bundle(pending.provider, &bundle).await?;
        Ok(bundle)
    }

    /// Completes an Android deep-link callback URI such as `vessel://oauth/anilist?code=...&state=...`.
    pub async fn complete_callback_uri(&self, callback_uri: &str) -> VesselResult<OAuthTokenBundle> {
        let parsed = Url::parse(callback_uri)
            .map_err(|err| VesselError::InvalidInput(format!("invalid oauth callback uri: {err}")))?;
        let mut code = None;
        let mut state = None;
        let mut err_name = None;
        for (k, v) in parsed.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => err_name = Some(v.into_owned()),
                _ => {}
            }
        }
        if let Some(err) = err_name {
            return Err(VesselError::Network(format!("oauth callback error: {err}")));
        }
        let code = code.ok_or_else(|| VesselError::InvalidInput("missing oauth code".into()))?;
        let state = state.ok_or_else(|| VesselError::InvalidInput("missing oauth state".into()))?;
        self.complete_code_callback(&state, &code).await
    }

    /// Starts an OAuth 2.0 Device Code flow (ideal for TV / QR login).
    pub async fn start_device_code_flow(
        &self,
        provider: OAuthProviderKind,
    ) -> VesselResult<OAuthDevicePrompt> {
        let spec = self.spec(provider)?;
        let device_url = spec.device_code_url.as_deref().ok_or_else(|| {
            VesselError::InvalidInput(format!(
                "{} does not support Device Code flow",
                provider.as_str()
            ))
        })?;
        let cfg = self.require_client_config(provider).await?;

        #[derive(Serialize)]
        struct DeviceReq<'a> {
            client_id: &'a str,
            #[serde(skip_serializing_if = "str::is_empty")]
            scope: &'a str,
        }

        #[derive(Deserialize)]
        struct DeviceResp {
            device_code: String,
            user_code: String,
            verification_url: Option<String>,
            verification_uri: Option<String>,
            verification_uri_complete: Option<String>,
            expires_in: Option<u64>,
            interval: Option<u64>,
        }

        let resp = if provider == OAuthProviderKind::GoogleDrive {
            let body = encode_form_pairs(&[
                ("client_id", cfg.client_id.as_str()),
                ("scope", spec.scopes.as_str()),
            ]);
            self.http
                .post(device_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
        } else {
            self.http
                .post(device_url)
                .json(&DeviceReq {
                    client_id: &cfg.client_id,
                    scope: &spec.scopes,
                })
                .send()
                .await
        }
        .map_err(|err| VesselError::Network(format!("device code request failed: {err}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read device code response: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "device code HTTP {status}: {}",
                text.trim()
            )));
        }

        let parsed: DeviceResp = serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode device code response: {err}")))?;
        let verification_uri = parsed
            .verification_uri
            .or(parsed.verification_url)
            .ok_or_else(|| VesselError::Network("device code response missing verification URL".into()))?;

        Ok(OAuthDevicePrompt {
            provider,
            device_code: parsed.device_code,
            user_code: parsed.user_code,
            verification_uri,
            verification_uri_complete: parsed.verification_uri_complete,
            expires_in_secs: parsed.expires_in.unwrap_or(600),
            interval_secs: parsed.interval.unwrap_or(5).max(1),
        })
    }

    /// Polls a Device Code once; returns `Ok(None)` while user authorization is still pending.
    pub async fn poll_device_code(
        &self,
        provider: OAuthProviderKind,
        device_code: &str,
    ) -> VesselResult<Option<OAuthTokenBundle>> {
        let spec = self.spec(provider)?;
        let token_url = spec
            .device_token_url
            .as_deref()
            .unwrap_or(spec.token_url.as_str());
        let cfg = self.require_client_config(provider).await?;

        let resp = if provider == OAuthProviderKind::GoogleDrive {
            let mut form: Vec<(&str, String)> = vec![
                ("client_id", cfg.client_id),
                ("device_code", device_code.to_owned()),
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".into(),
                ),
            ];
            if let Some(secret) = cfg.client_secret {
                form.push(("client_secret", secret));
            }
            let body = encode_form_owned(&form);
            self.http
                .post(token_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
        } else {
            let mut body = serde_json::json!({
                "code": device_code,
                "client_id": cfg.client_id,
            });
            if let Some(secret) = cfg.client_secret {
                body["client_secret"] = serde_json::Value::String(secret);
            }
            self.http.post(token_url).json(&body).send().await
        }
        .map_err(|err| VesselError::Network(format!("device token poll failed: {err}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read device token response: {err}")))?;

        if status.is_success() {
            let bundle = parse_token_response(&text, None)?;
            self.save_token_bundle(provider, &bundle).await?;
            return Ok(Some(bundle));
        }

        if status.as_u16() == 400 || status.as_u16() == 428 {
            if text.contains("authorization_pending") || text.contains("slow_down") {
                return Ok(None);
            }
        }
        Err(VesselError::Network(format!(
            "device token poll HTTP {status}: {}",
            text.trim()
        )))
    }

    async fn refresh_with_token(
        &self,
        provider: OAuthProviderKind,
        refresh_token: &str,
    ) -> VesselResult<OAuthTokenBundle> {
        let cfg = self.require_client_config(provider).await?;
        let spec = self.spec(provider)?;
        let mut form: Vec<(&str, String)> = vec![
            ("grant_type", "refresh_token".into()),
            ("refresh_token", refresh_token.to_owned()),
            ("client_id", cfg.client_id),
        ];
        if let Some(secret) = cfg.client_secret {
            form.push(("client_secret", secret));
        }
        self.exchange_token_form_with_fallback(&spec.token_url, &form, Some(refresh_token))
            .await
    }

    async fn exchange_token_form(
        &self,
        token_url: &str,
        form: &[(&str, String)],
    ) -> VesselResult<OAuthTokenBundle> {
        self.exchange_token_form_with_fallback(token_url, form, None)
            .await
    }

    async fn exchange_token_form_with_fallback(
        &self,
        token_url: &str,
        form: &[(&str, String)],
        fallback_refresh: Option<&str>,
    ) -> VesselResult<OAuthTokenBundle> {
        let body = encode_form_owned(form);
        let resp = self
            .http
            .post(token_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("token exchange failed: {err}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("read token response: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "token exchange HTTP {status}: {}",
                text.trim()
            )));
        }
        parse_token_response(&text, fallback_refresh)
    }

    async fn require_client_config(&self, provider: OAuthProviderKind) -> VesselResult<OAuthClientConfig> {
        self.get_client_config(provider).await?.ok_or_else(|| {
            VesselError::InvalidInput(format!(
                "OAuth clientId is not configured for {}",
                provider.as_str()
            ))
        })
    }

    fn spec(&self, provider: OAuthProviderKind) -> VesselResult<&OAuthProviderSpec> {
        self.specs.get(&provider).ok_or_else(|| {
            VesselError::NotFound(format!("missing OAuth spec for {}", provider.as_str()))
        })
    }

    async fn provider_refresh_lock(&self, provider: OAuthProviderKind) -> Arc<Mutex<()>> {
        let mut map = self.refresh_locks.lock().await;
        Arc::clone(map.entry(provider).or_insert_with(|| Arc::new(Mutex::new(()))))
    }
}

fn encode_form_pairs(pairs: &[(&str, &str)]) -> String {
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        ser.append_pair(k, v);
    }
    ser.finish()
}

fn encode_form_owned(pairs: &[(&str, String)]) -> String {
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        ser.append_pair(k, v);
    }
    ser.finish()
}

fn parse_token_response(
    raw_json: &str,
    fallback_refresh: Option<&str>,
) -> VesselResult<OAuthTokenBundle> {
    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
        refresh_token: Option<String>,
        token_type: Option<String>,
        scope: Option<String>,
        expires_in: Option<i64>,
    }

    let parsed: TokenResp = serde_json::from_str(raw_json)
        .map_err(|err| VesselError::Network(format!("decode token JSON: {err}")))?;
    let expires_at_epoch_secs = parsed
        .expires_in
        .map(|secs| now_epoch_secs().saturating_add(secs));
    Ok(OAuthTokenBundle {
        access_token: parsed.access_token,
        refresh_token: parsed
            .refresh_token
            .or_else(|| fallback_refresh.map(str::to_owned)),
        token_type: parsed.token_type.unwrap_or_else(|| "Bearer".into()),
        scope: parsed.scope,
        expires_at_epoch_secs,
    })
}

fn pkce_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn random_urlsafe(byte_len: usize) -> VesselResult<String> {
    let mut buf = vec![0u8; byte_len];
    getrandom::fill(&mut buf)
        .map_err(|err| VesselError::Storage(format!("generate oauth random bytes: {err}")))?;
    Ok(URL_SAFE_NO_PAD.encode(&buf))
}

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::Form;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::{Json, Router};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use vessel_domain::{
        OAuthClientConfig, OAuthProviderKind, OAuthStartResult, OAuthTokenBundle, SecretStore,
        VesselResult,
    };

    use super::{OAuthManager, OAuthProviderSpec};

    #[derive(Default)]
    struct MemSecrets {
        map: Mutex<BTreeMap<String, String>>,
    }

    #[async_trait::async_trait]
    impl SecretStore for MemSecrets {
        async fn get(&self, key: &str) -> VesselResult<Option<String>> {
            Ok(self.map.lock().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
            self.map
                .lock()
                .await
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        async fn delete(&self, key: &str) -> VesselResult<()> {
            self.map.lock().await.remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn exchanges_pkce_loopback_and_performs_single_flight_refresh() {
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let refresh_counter = Arc::clone(&refresh_calls);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new()
            .route(
                "/token",
                post(move |Form(form): Form<Vec<(String, String)>>| {
                    let counter = Arc::clone(&refresh_counter);
                    async move {
                        let map: BTreeMap<String, String> = form.into_iter().collect();
                        if map.get("grant_type").map(String::as_str) == Some("refresh_token") {
                            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
                            return (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "access_token": format!("refreshed-tok-{n}"),
                                    "refresh_token": "ref-2",
                                    "expires_in": 3600
                                })),
                            );
                        }
                        assert!(map.contains_key("code_verifier"));
                        (
                            StatusCode::OK,
                            Json(serde_json::json!({
                                "access_token": "initial-tok",
                                "refresh_token": "ref-1",
                                "expires_in": 10
                            })),
                        )
                    }
                }),
            )
            .route(
                "/device",
                post(|| async {
                    Json(serde_json::json!({
                        "device_code": "dev-123",
                        "user_code": "ABCD-1234",
                        "verification_uri": "https://trakt.tv/activate",
                        "expires_in": 600,
                        "interval": 5
                    }))
                }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let base = format!("http://{addr}");
        let secrets = Arc::new(MemSecrets::default());
        let mgr = Arc::new(
            OAuthManager::new(secrets)
                .expect("oauth manager")
                .with_custom_spec(OAuthProviderSpec {
                    kind: OAuthProviderKind::Trakt,
                    authorize_url: format!("{base}/auth"),
                    token_url: format!("{base}/token"),
                    device_code_url: Some(format!("{base}/device")),
                    device_token_url: Some(format!("{base}/token")),
                    scopes: String::new(),
                    supports_pkce: true,
                    plain_pkce_only: false,
                    supports_device_code: true,
                }),
        );

        mgr.save_client_config(
            OAuthProviderKind::Trakt,
            OAuthClientConfig {
                client_id: "cid-1".into(),
                client_secret: Some("csec-1".into()),
            },
        )
        .await
        .expect("save client config");

        let (start, rx) = mgr
            .start_loopback_pkce_session(OAuthProviderKind::Trakt)
            .await
            .expect("start loopback");
        let OAuthStartResult::BrowserUrl {
            auth_url,
            redirect_uri,
            state,
            ..
        } = start
        else {
            panic!("expected browser url");
        };
        assert!(auth_url.contains("code_challenge_method=S256"));

        let cb_client = reqwest::Client::new();
        let resp = cb_client
            .get(format!("{redirect_uri}?code=auth-code-1&state={state}"))
            .send()
            .await
            .expect("hit loopback");
        assert!(resp.status().is_success());

        let bundle: OAuthTokenBundle = rx
            .await
            .expect("oneshot")
            .expect("token exchange succeeded");
        assert_eq!(bundle.access_token, "initial-tok");

        // expires_in=10 is inside the 120s skew window, so concurrent readers share 1 refresh.
        let (t1, t2) = tokio::join!(
            mgr.get_valid_access_token(OAuthProviderKind::Trakt),
            mgr.get_valid_access_token(OAuthProviderKind::Trakt),
        );
        assert_eq!(t1.expect("t1"), "refreshed-tok-1");
        assert_eq!(t2.expect("t2"), "refreshed-tok-1");
        assert_eq!(refresh_calls.load(Ordering::SeqCst), 1);

        let prompt = mgr
            .start_device_code_flow(OAuthProviderKind::Trakt)
            .await
            .expect("device code");
        assert_eq!(prompt.user_code, "ABCD-1234");
    }
}
