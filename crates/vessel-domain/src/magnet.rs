//! Magnet URI and BitTorrent info-hash parsing and normalization utilities.

use crate::{VesselError, VesselResult};

/// Parsed BitTorrent magnet link with normalized lowercase 40-char hex `info_hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMagnet {
    pub info_hash: String,
    pub display_name: Option<String>,
    pub trackers: Vec<String>,
}

/// Decodes a Base32 (RFC 4648) string into raw bytes without external dependencies.
fn decode_base32(input: &str) -> Option<Vec<u8>> {
    let clean = input.trim().trim_end_matches('=');
    if clean.is_empty() {
        return None;
    }

    let mut out = Vec::with_capacity((clean.len() * 5) / 8);
    let mut buffer: u32 = 0;
    let mut bits_left: u8 = 0;

    for ch in clean.bytes() {
        let val: u32 = match ch {
            b'A'..=b'Z' => (ch - b'A') as u32,
            b'a'..=b'z' => (ch - b'a') as u32,
            b'2'..=b'7' => (ch - b'2' + 26) as u32,
            _ => return None,
        };
        buffer = (buffer << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            out.push(((buffer >> bits_left) & 0xff) as u8);
        }
    }

    Some(out)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (
                    (bytes[i + 1] as char).to_digit(16),
                    (bytes[i + 2] as char).to_digit(16),
                ) {
                    out.push(((hi << 4) | lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else if byte == b' ' {
            out.push_str("%20");
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Normalizes a 40-char hex or 32-char Base32 BitTorrent info-hash into lowercase 40-char hex.
pub fn normalize_info_hash(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() == 40 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Some(trimmed.to_ascii_lowercase());
    }
    if trimmed.len() == 32 {
        let bytes = decode_base32(trimmed)?;
        if bytes.len() == 20 {
            let mut hex = String::with_capacity(40);
            for b in bytes {
                hex.push_str(&format!("{b:02x}"));
            }
            return Some(hex);
        }
    }
    None
}

/// Extracts a normalized 40-char hex info-hash from either a `magnet:?` URI or a raw hash string.
pub fn extract_info_hash(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if let Some(norm) = normalize_info_hash(trimmed) {
        return Some(norm);
    }
    parse_magnet_uri(trimmed).ok().map(|p| p.info_hash)
}

/// Parses a `magnet:?xt=urn:btih:...` URI and extracts normalized hash, display name, and trackers.
pub fn parse_magnet_uri(raw: &str) -> VesselResult<ParsedMagnet> {
    let trimmed = raw.trim();
    let query = trimmed
        .strip_prefix("magnet:?")
        .or_else(|| trimmed.strip_prefix("MAGNET:?"))
        .ok_or_else(|| {
            VesselError::InvalidInput(format!("not a valid magnet URI: '{trimmed}'"))
        })?;

    let mut info_hash: Option<String> = None;
    let mut display_name: Option<String> = None;
    let mut trackers = Vec::new();

    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (key, val) = part.split_once('=').unwrap_or((part, ""));
        match key.to_ascii_lowercase().as_str() {
            "xt" => {
                let decoded = percent_decode(val);
                if let Some(hash_raw) = decoded
                    .strip_prefix("urn:btih:")
                    .or_else(|| decoded.strip_prefix("URN:BTIH:"))
                    && let Some(norm) = normalize_info_hash(hash_raw)
                {
                    info_hash = Some(norm);
                }
            }
            "dn" if display_name.is_none() => {
                let decoded = percent_decode(val).trim().to_owned();
                if !decoded.is_empty() {
                    display_name = Some(decoded);
                }
            }
            "tr" => {
                let decoded = percent_decode(val).trim().to_owned();
                if !decoded.is_empty() && !trackers.contains(&decoded) {
                    trackers.push(decoded);
                }
            }
            _ => {}
        }
    }

    let info_hash = info_hash.ok_or_else(|| {
        VesselError::InvalidInput(format!(
            "magnet URI missing valid urn:btih infoHash: '{trimmed}'"
        ))
    })?;

    Ok(ParsedMagnet {
        info_hash,
        display_name,
        trackers,
    })
}

/// Builds a canonical `magnet:?xt=urn:btih:...` URI from a hash, optional display name, and trackers.
pub fn build_magnet_uri(
    info_hash: &str,
    display_name: Option<&str>,
    trackers: &[String],
) -> String {
    let norm = normalize_info_hash(info_hash).unwrap_or_else(|| info_hash.trim().to_owned());
    let mut out = format!("magnet:?xt=urn:btih:{norm}");
    if let Some(dn) = display_name.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str("&dn=");
        out.push_str(&percent_encode_component(dn));
    }
    for tr in trackers {
        let tr = tr.trim();
        let clean = tr.strip_prefix("tracker:").unwrap_or(tr);
        if !clean.is_empty() {
            out.push_str("&tr=");
            out.push_str(&percent_encode_component(clean));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{build_magnet_uri, extract_info_hash, normalize_info_hash, parse_magnet_uri};

    #[test]
    fn parses_hex_and_base32_magnet_uris() {
        let hex_magnet = "magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01&dn=Movie%202026&tr=udp%3A%2F%2Ftracker.example%3A80";
        let parsed = parse_magnet_uri(hex_magnet).expect("parse hex magnet");
        assert_eq!(parsed.info_hash, "abcdef0123456789abcdef0123456789abcdef01");
        assert_eq!(parsed.display_name.as_deref(), Some("Movie 2026"));
        assert_eq!(parsed.trackers, vec!["udp://tracker.example:80"]);

        // 32-char Base32 for 20 bytes of 0xAA (`0b10101010` -> `V` (21) + `K` (10) repeating)
        let b32_hash = "VKVKVKVKVKVKVKVKVKVKVKVKVKVKVKVK";
        let norm = normalize_info_hash(b32_hash).expect("decode base32");
        assert_eq!(norm, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(
            extract_info_hash(&format!("magnet:?xt=urn:btih:{b32_hash}")),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into())
        );

        let rebuilt = build_magnet_uri(
            "ABCDEF0123456789ABCDEF0123456789ABCDEF01",
            Some("Show S01E01"),
            &["tracker:udp://t.example:1337".into()],
        );
        assert!(rebuilt.starts_with("magnet:?xt=urn:btih:abcdef0123456789"));
        assert!(rebuilt.contains("&dn=Show%20S01E01"));
        assert!(rebuilt.contains("&tr=udp%3A%2F%2Ft.example%3A1337"));
    }
}
