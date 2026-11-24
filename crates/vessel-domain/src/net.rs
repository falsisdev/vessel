//! Shared `reqwest` client builder configured for all Vessel crates.
//!
//! On Android, `rustls-platform-verifier`'s `PKIXRevocationChecker` invokes Android's
//! `CertPathValidator`, which rejects valid Let's Encrypt and CDN chains whose CRL URIs use
//! cleartext `http://` (blocked by default `network_security_config`) with `Revoked`.
//! Preconfiguring `rustls` with Mozilla's `webpki-roots` avoids false revocation failures while
//! keeping full TLS certificate and hostname verification active across all crates.

/// Returns a [`reqwest::ClientBuilder`] with Vessel's platform-appropriate TLS configuration.
pub fn http_client_builder() -> reqwest::ClientBuilder {
    #[cfg(target_os = "android")]
    {
        reqwest::Client::builder().tls_backend_preconfigured(android_rustls_config())
    }
    #[cfg(not(target_os = "android"))]
    {
        reqwest::Client::builder()
    }
}

#[cfg(target_os = "android")]
fn android_rustls_config() -> rustls::ClientConfig {
    use std::sync::{Arc, OnceLock};

    static CONFIG: OnceLock<rustls::ClientConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
            let mut cfg = rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("valid default rustls TLS protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
            cfg
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::http_client_builder;

    #[test]
    fn builds_shared_http_client() {
        assert!(http_client_builder().build().is_ok());
    }
}
