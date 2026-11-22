//! SSRF network guards and per-plugin HTTPS domain permission policy.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::RwLock;

use url::{Host, Url};
use vessel_domain::{PluginId, VesselError, VesselResult};

const DOMAIN_BLOCKED_PREFIX: &str = "[VesselDomainBlockedError]: plugin '";
const DOMAIN_BLOCKED_MIDDLE: &str = "' requires permission for domain '";

/// Parses a formatted `[VesselDomainBlockedError]` string back into `VesselError::DomainBlocked`.
pub(crate) fn parse_domain_blocked_message(message: &str) -> Option<VesselError> {
    let start = message.find(DOMAIN_BLOCKED_PREFIX)?;
    let rest = &message[start + DOMAIN_BLOCKED_PREFIX.len()..];
    let mid = rest.find(DOMAIN_BLOCKED_MIDDLE)?;
    let plugin_id = &rest[..mid];
    let after_mid = &rest[mid + DOMAIN_BLOCKED_MIDDLE.len()..];
    let end = after_mid.find('\'')?;
    let domain = &after_mid[..end];
    if plugin_id.is_empty() || domain.is_empty() {
        return None;
    }
    Some(VesselError::DomainBlocked {
        plugin_id: plugin_id.to_owned(),
        domain: domain.to_owned(),
    })
}

/// Returns `true` if `ip` belongs to a loopback, private, link-local, CGNAT, or reserved non-public range.
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    let is_cgnat = octets[0] == 100 && (64..=127).contains(&octets[1]);
    let is_benchmarking = octets[0] == 198 && (18..=19).contains(&octets[1]);
    let is_this_network = octets[0] == 0;
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_documentation()
        || is_cgnat
        || is_benchmarking
        || is_this_network
}

fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_forbidden_ipv4(mapped);
    }
    let segments = ip.segments();
    let is_unique_local = (segments[0] & 0xfe00) == 0xfc00;
    let is_unicast_link_local = (segments[0] & 0xffc0) == 0xfe80;
    let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || is_unique_local
        || is_unicast_link_local
        || is_documentation
}

fn is_forbidden_hostname(host: &str) -> bool {
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
        || normalized.ends_with(".arpa")
}

fn domain_matches(pattern: &str, host: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host.ends_with(&format!(".{suffix}"));
    }
    pattern == host
}

/// Per-plugin outbound network policy combining immutable SSRF blocks with domain allowlisting.
#[derive(Debug)]
pub struct DomainPolicy {
    plugin_id: PluginId,
    allowed_domains: RwLock<HashSet<String>>,
    allow_any_https: bool,
    allow_loopback_for_tests: bool,
}

impl DomainPolicy {
    /// Builds a policy from manifest `domains` and `allow_any_https`, plus any persisted grants.
    pub fn new(
        plugin_id: PluginId,
        initial_domains: impl IntoIterator<Item = String>,
        allow_any_https: bool,
    ) -> Self {
        let normalized = initial_domains
            .into_iter()
            .map(|domain| domain.trim().trim_end_matches('.').to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect();
        Self {
            plugin_id,
            allowed_domains: RwLock::new(normalized),
            allow_any_https,
            allow_loopback_for_tests: false,
        }
    }

    /// Test-only helper enabling local `axum` mock servers on `127.0.0.1` / `::1`.
    #[cfg(test)]
    pub(crate) fn with_loopback_for_tests(mut self) -> Self {
        self.allow_loopback_for_tests = true;
        self
    }

    /// Dynamically authorizes an additional domain for this plugin after explicit user consent.
    pub fn allow_domain(&self, domain: &str) -> VesselResult<()> {
        let normalized = domain.trim().trim_end_matches('.').to_ascii_lowercase();
        if normalized.is_empty() || is_forbidden_hostname(&normalized) {
            return Err(VesselError::InvalidInput(format!(
                "invalid domain grant '{domain}'"
            )));
        }
        if let Ok(ip) = normalized.parse::<IpAddr>()
            && is_forbidden_ip(ip)
        {
            return Err(VesselError::InvalidInput(format!(
                "cannot allow private or loopback IP '{domain}'"
            )));
        }

        let mut guard = self
            .allowed_domains
            .write()
            .map_err(|err| VesselError::Storage(format!("domain policy write lock: {err}")))?;
        guard.insert(normalized);
        Ok(())
    }

    /// Returns whether `host` is currently permitted under this plugin's policy.
    pub fn is_domain_allowed(&self, host: &str, is_https: bool) -> VesselResult<bool> {
        let normalized = host.trim_end_matches('.').to_ascii_lowercase();
        let guard = self
            .allowed_domains
            .read()
            .map_err(|err| VesselError::Storage(format!("domain policy read lock: {err}")))?;
        if guard
            .iter()
            .any(|pattern| domain_matches(pattern, &normalized))
        {
            return Ok(true);
        }
        Ok(self.allow_any_https && is_https)
    }

    /// Validates an outbound URL against scheme, SSRF blocks, and the plugin domain allowlist.
    pub fn authorize_url(&self, raw_url: &str) -> VesselResult<Url> {
        let parsed = Url::parse(raw_url)
            .map_err(|err| VesselError::InvalidInput(format!("invalid url '{raw_url}': {err}")))?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(VesselError::InvalidInput(format!(
                "unsupported URL scheme '{scheme}'; only http/https are permitted"
            )));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(VesselError::InvalidInput(
                "credentials embedded in URL userinfo are not permitted".into(),
            ));
        }

        let host = parsed
            .host()
            .ok_or_else(|| VesselError::InvalidInput(format!("URL missing host: '{raw_url}'")))?;
        let host_str = parsed
            .host_str()
            .ok_or_else(|| VesselError::InvalidInput(format!("URL missing host: '{raw_url}'")))?;

        let is_test_loopback = self.allow_loopback_for_tests
            && match host {
                Host::Ipv4(v4) => v4.is_loopback(),
                Host::Ipv6(v6) => v6.is_loopback(),
                Host::Domain(_) => false,
            };

        if !is_test_loopback {
            match host {
                Host::Ipv4(v4) if is_forbidden_ipv4(v4) => {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked SSRF IPv4 host '{host_str}'"
                    )));
                }
                Host::Ipv6(v6) if is_forbidden_ipv6(v6) => {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked SSRF IPv6 host '{host_str}'"
                    )));
                }
                Host::Domain(domain) if is_forbidden_hostname(domain) => {
                    return Err(VesselError::InvalidInput(format!(
                        "blocked local hostname '{host_str}'"
                    )));
                }
                _ => {}
            }
        }

        let normalized_host = host_str
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_end_matches('.')
            .to_ascii_lowercase();

        if !self.is_domain_allowed(&normalized_host, scheme == "https")? {
            return Err(VesselError::DomainBlocked {
                plugin_id: self.plugin_id.to_string(),
                domain: normalized_host,
            });
        }

        Ok(parsed)
    }

    /// Ensures DNS-resolved target socket addresses do not point to internal/private ranges.
    pub fn validate_resolved_addrs(&self, host: &str, addrs: &[SocketAddr]) -> VesselResult<()> {
        if self.allow_loopback_for_tests && addrs.iter().all(|addr| addr.ip().is_loopback()) {
            return Ok(());
        }
        for addr in addrs {
            if is_forbidden_ip(addr.ip()) {
                return Err(VesselError::InvalidInput(format!(
                    "blocked SSRF resolved address '{}' for host '{host}'",
                    addr.ip()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use vessel_domain::{PluginId, VesselError};

    use super::{DomainPolicy, is_forbidden_ip, parse_domain_blocked_message};

    fn anthology_id() -> PluginId {
        PluginId::new("anthology").expect("valid plugin id")
    }

    #[test]
    fn blocks_ssrf_targets_even_when_allow_any_https_is_enabled() {
        let policy = DomainPolicy::new(anthology_id(), vec![], true);

        for blocked in [
            "http://127.0.0.1/admin",
            "https://127.0.0.1:8443/api",
            "https://[::1]/secret",
            "https://10.0.0.1/router",
            "https://172.16.0.5/internal",
            "https://192.168.1.1/",
            "https://169.254.169.254/latest/meta-data/",
            "https://localhost:3000/status",
            "https://api.localhost/v1",
            "https://nas.local/media",
            "file:///etc/passwd",
            "https://user:pass@anthology.example/",
        ] {
            let err = policy
                .authorize_url(blocked)
                .expect_err(&format!("expected SSRF block for {blocked}"));
            assert!(
                matches!(err, VesselError::InvalidInput(_)),
                "unexpected error for {blocked}: {err:?}"
            );
        }

        assert!(is_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_forbidden_ip(IpAddr::V6(Ipv6Addr::new(
            0xfd00, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(!is_forbidden_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

        // Rebound DNS resolution to a loopback IP must be blocked too.
        let dns_err = policy
            .validate_resolved_addrs(
                "rebind.anthology.example",
                &[SocketAddr::from(([127, 0, 0, 1], 443))],
            )
            .expect_err("reject private resolved address");
        assert!(matches!(dns_err, VesselError::InvalidInput(_)));
    }

    #[test]
    fn enforces_domain_allowlist_and_supports_dynamic_grants() {
        let policy = DomainPolicy::new(
            anthology_id(),
            vec!["anthology.example".into(), "*.cdn.anthology.example".into()],
            false,
        );

        assert!(
            policy
                .authorize_url("https://anthology.example/catalog")
                .is_ok()
        );
        assert!(
            policy
                .authorize_url("https://edge1.cdn.anthology.example/master.m3u8")
                .is_ok()
        );

        let err = policy
            .authorize_url("https://mirror.anthology.example/watch")
            .expect_err("reject unapproved mirror domain");
        assert_eq!(
            err,
            VesselError::DomainBlocked {
                plugin_id: "anthology".into(),
                domain: "mirror.anthology.example".into(),
            }
        );

        // Parse helper reconstructs typed DomainBlocked from formatted JS exception messages.
        let msg = format!("Error: {err}");
        assert_eq!(parse_domain_blocked_message(&msg), Some(err));

        policy
            .allow_domain("mirror.anthology.example")
            .expect("grant dynamic domain");
        assert!(
            policy
                .authorize_url("https://mirror.anthology.example/watch")
                .is_ok()
        );
    }
}
