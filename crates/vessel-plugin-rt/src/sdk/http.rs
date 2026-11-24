//! `vessel.http` SSRF-safe HTTP client bridged from Tokio `reqwest` (`rustls`) into QuickJS Promises.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, LOCATION, USER_AGENT};
use rquickjs::function::{IntoJsFunc, ParamRequirement, Params};
use rquickjs::prelude::Async;
use rquickjs::{
    CatchResultExt, Ctx, Function, IntoJs, Object, Result as QjsResult, TypedArray, Value,
};
use url::Url;
use vessel_domain::{VesselError, VesselResult};

use super::{HostResult, HostServices};

const DEFAULT_HTTP_TIMEOUT_MS: u64 = 15_000;
const MAX_HTTP_TIMEOUT_MS: u64 = 30_000;
const MAX_REDIRECTS: usize = 10;

const HTTP_WRAPPER_JS: &str = r#"
(() => {
    const rawReq = globalThis.__vessel_http_request;
    if (typeof rawReq !== "function") {
        throw new Error("missing __vessel_http_request binding");
    }
    const normalizeOpts = (opts) => (opts && typeof opts === "object") ? opts : {};
    const send = (method, url, body, opts) => {
        const o = normalizeOpts(opts);
        const headers = (o.headers && typeof o.headers === "object") ? { ...o.headers } : {};
        let payload = null;
        if (body !== undefined && body !== null) {
            if (typeof body === "string") {
                payload = body;
            } else {
                payload = JSON.stringify(body);
                const hasCt = Object.keys(headers).some((k) => k.toLowerCase() === "content-type");
                if (!hasCt) headers["Content-Type"] = "application/json";
            }
        }
        const timeoutMs = typeof o.timeoutMs === "number" ? o.timeoutMs : 15000;
        const followRedirects = o.followRedirects !== false;
        return rawReq(method, String(url), JSON.stringify(headers), payload, timeoutMs, followRedirects);
    };
    globalThis.vessel.http = {
        get: (url, opts) => send("GET", url, null, opts),
        post: (url, body, opts) => send("POST", url, body, opts),
        request: (opts) => {
            const o = normalizeOpts(opts);
            return send(String(o.method || "GET"), o.url, o.body, o);
        },
    };
})();
"#;

struct ExecutedResponse {
    status: u16,
    final_url: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

struct ResponseBytesFn(Arc<Vec<u8>>);

impl<'js> IntoJsFunc<'js, ()> for ResponseBytesFn {
    fn param_requirements() -> ParamRequirement {
        ParamRequirement::none()
    }

    fn call(&self, params: Params<'_, 'js>) -> QjsResult<Value<'js>> {
        let ctx = params.ctx().clone();
        TypedArray::new_copy(ctx.clone(), self.0.as_slice())?.into_js(&ctx)
    }
}

fn parse_json_str<'js>(ctx: Ctx<'js>, raw: String) -> QjsResult<Value<'js>> {
    ctx.json_parse(raw)
}

impl<'js> IntoJs<'js> for ExecutedResponse {
    fn into_js(self, ctx: &Ctx<'js>) -> QjsResult<Value<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("status", self.status)?;
        obj.set("url", self.final_url)?;

        let headers_obj = Object::new(ctx.clone())?;
        for (k, v) in self.headers {
            headers_obj.set(k, v)?;
        }
        obj.set("headers", headers_obj)?;

        let body = Arc::new(self.body);
        {
            let body = Arc::clone(&body);
            obj.set(
                "text",
                Function::new(ctx.clone(), move || -> String {
                    String::from_utf8_lossy(&body).into_owned()
                })?,
            )?;
        }
        {
            let body = Arc::clone(&body);
            let parse_fn = Function::new(ctx.clone(), parse_json_str)?;
            obj.set(
                "json",
                Function::new(ctx.clone(), move || -> QjsResult<Value<'_>> {
                    let text = std::str::from_utf8(&body).map_err(|err| {
                        rquickjs::Error::new_from_js_message("bytes", "utf8", err.to_string())
                    })?;
                    parse_fn.call((text,))
                })?,
            )?;
        }
        obj.set("bytes", Function::new(ctx.clone(), ResponseBytesFn(body))?)?;

        Ok(obj.into_value())
    }
}

async fn validate_dns_target(host: &HostServices, url: &Url) -> VesselResult<()> {
    let Some(hostname) = url.host_str() else {
        return Err(VesselError::InvalidInput("URL missing host".into()));
    };
    let port = url.port_or_known_default().unwrap_or(80);
    if hostname
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<std::net::IpAddr>()
        .is_ok()
    {
        return Ok(());
    }

    let resolved: Vec<std::net::SocketAddr> = tokio::net::lookup_host((hostname, port))
        .await
        .map_err(|err| VesselError::Network(format!("DNS lookup failed for '{hostname}': {err}")))?
        .collect();
    host.domain_policy
        .validate_resolved_addrs(hostname, &resolved)
}

async fn execute_http_request(
    host: Arc<HostServices>,
    method_raw: String,
    url_raw: String,
    headers_json: String,
    mut body: Option<String>,
    timeout_ms: u64,
    follow_redirects: bool,
) -> VesselResult<ExecutedResponse> {
    let mut method = Method::from_bytes(method_raw.trim().to_ascii_uppercase().as_bytes())
        .map_err(|err| VesselError::InvalidInput(format!("invalid HTTP method: {err}")))?;
    let mut current_url = host.domain_policy.authorize_url(&url_raw)?;
    let headers_map: BTreeMap<String, String> = serde_json::from_str(&headers_json)
        .map_err(|err| VesselError::InvalidInput(format!("invalid request headers: {err}")))?;

    let mut header_map = HeaderMap::new();
    for (name, val) in &headers_map {
        let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|err| {
            VesselError::InvalidInput(format!("invalid request header '{name}': {err}"))
        })?;
        let header_value = HeaderValue::from_str(val).map_err(|err| {
            VesselError::InvalidInput(format!("invalid value for header '{name}': {err}"))
        })?;
        header_map.insert(header_name, header_value);
    }
    if !header_map.contains_key(USER_AGENT) {
        header_map.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            ),
        );
    }

    let timeout = Duration::from_millis(timeout_ms.clamp(1, MAX_HTTP_TIMEOUT_MS).max(
        if timeout_ms == 0 {
            DEFAULT_HTTP_TIMEOUT_MS
        } else {
            1
        },
    ));

    for hop in 0..=MAX_REDIRECTS {
        validate_dns_target(&host, &current_url).await?;

        let mut req = host
            .http_client
            .request(method.clone(), current_url.clone())
            .headers(header_map.clone())
            .timeout(timeout);
        if let Some(ref payload) = body {
            req = req.body(payload.clone());
        }

        let resp = req.send().await.map_err(|err| {
            VesselError::Network(format!(
                "http request to {current_url}: {}",
                vessel_domain::format_error_chain(&err)
            ))
        })?;

        if follow_redirects && resp.status().is_redirection() {
            if hop == MAX_REDIRECTS {
                return Err(VesselError::Network(format!(
                    "too many HTTP redirects (> {MAX_REDIRECTS})"
                )));
            }
            let Some(location) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok()) else {
                return Err(VesselError::Network(
                    "redirect response missing Location header".into(),
                ));
            };
            let next_url = current_url.join(location).map_err(|err| {
                VesselError::InvalidInput(format!("invalid redirect target '{location}': {err}"))
            })?;
            current_url = host.domain_policy.authorize_url(next_url.as_str())?;

            if matches!(resp.status().as_u16(), 301..=303) && method == Method::POST {
                method = Method::GET;
                body = None;
            }
            continue;
        }

        let status = resp.status().as_u16();
        let mut resp_headers = BTreeMap::new();
        for (k, v) in resp.headers() {
            if let Ok(text) = v.to_str() {
                resp_headers.insert(k.as_str().to_ascii_lowercase(), text.to_owned());
            }
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|err| VesselError::Network(format!("read response body: {err}")))?
            .to_vec();

        return Ok(ExecutedResponse {
            status,
            final_url: current_url.to_string(),
            headers: resp_headers,
            body: bytes,
        });
    }

    unreachable!("redirect loop always returns or errors by MAX_REDIRECTS")
}

/// Attaches `vessel.http` (`get`, `post`, `request`) to `globalThis.vessel`.
pub(crate) fn attach_http<'js>(
    ctx: &Ctx<'js>,
    host: Arc<HostServices>,
    _vessel_obj: &Object<'js>,
) -> QjsResult<()> {
    ctx.globals().set(
        "__vessel_http_request",
        Function::new(
            ctx.clone(),
            Async(
                move |method: String,
                      url: String,
                      headers_json: String,
                      body: Option<String>,
                      timeout_ms: u64,
                      follow_redirects: bool| {
                    let host = Arc::clone(&host);
                    async move {
                        HostResult(
                            execute_http_request(
                                host,
                                method,
                                url,
                                headers_json,
                                body,
                                timeout_ms,
                                follow_redirects,
                            )
                            .await
                            .map_err(|err| err.to_string()),
                        )
                    }
                },
            ),
        )?,
    )?;

    ctx.eval::<(), _>(HTTP_WRAPPER_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;

    Ok(())
}
