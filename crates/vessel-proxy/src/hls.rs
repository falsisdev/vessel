//! HLS (`.m3u8`) playlist detection, relative URL resolution, and HMAC-signed rewriting.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use url::Url;
use vessel_domain::{VesselError, VesselResult};

use crate::HmacSigner;

/// Returns `true` when the target URL or upstream `Content-Type` indicates an HLS playlist.
pub fn is_hls_playlist(target_url: &str, content_type: Option<&str>) -> bool {
    if let Some(ct) = content_type {
        let lower = ct.to_ascii_lowercase();
        if lower.contains("mpegurl") || lower.contains("x-mpegurl") {
            return true;
        }
    }
    let path_without_query = target_url.split(['?', '#']).next().unwrap_or(target_url);
    path_without_query.to_ascii_lowercase().ends_with(".m3u8")
}

/// Builds an HMAC-signed `/s/{token}/seg?u=...&sig=...` proxy URL for a resolved upstream URL.
pub fn build_signed_seg_url(
    proxy_base_url: &str,
    token: &str,
    signer: &HmacSigner,
    resolved_url: &str,
) -> String {
    let encoded = URL_SAFE_NO_PAD.encode(resolved_url.as_bytes());
    let sig = signer.sign(token, &encoded);
    format!("{proxy_base_url}/s/{token}/seg?u={encoded}&sig={sig}")
}

/// Verifies `&sig=` in constant time and decodes `&u=` back into an HTTP(S) upstream URL.
pub fn decode_signed_seg_url(
    signer: &HmacSigner,
    token: &str,
    encoded_u: &str,
    sig: &str,
) -> Option<String> {
    if !signer.verify(token, encoded_u, sig) {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(encoded_u).ok()?;
    let url_str = String::from_utf8(decoded).ok()?;
    if !(url_str.starts_with("http://") || url_str.starts_with("https://")) {
        return None;
    }
    Some(url_str)
}

/// Rewrites relative/absolute segment, variant, key, map, and media URIs inside an `.m3u8` playlist.
pub fn rewrite_m3u8_playlist(
    playlist: &str,
    playlist_base_url: &str,
    proxy_base_url: &str,
    token: &str,
    signer: &HmacSigner,
) -> VesselResult<String> {
    let base = Url::parse(playlist_base_url).map_err(|err| {
        VesselError::Network(format!(
            "parse playlist base url '{playlist_base_url}': {err}"
        ))
    })?;

    let mut out = Vec::new();
    for raw_line in playlist.lines() {
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push(String::new());
        } else if trimmed.starts_with('#') {
            out.push(rewrite_tag_uri_attr(
                trimmed,
                &base,
                proxy_base_url,
                token,
                signer,
            )?);
        } else {
            let resolved = resolve_url(&base, trimmed)?;
            out.push(build_signed_seg_url(
                proxy_base_url,
                token,
                signer,
                resolved.as_str(),
            ));
        }
    }

    let mut result = out.join("\n");
    if playlist.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn resolve_url(base: &Url, reference: &str) -> VesselResult<Url> {
    base.join(reference)
        .map_err(|err| VesselError::Network(format!("resolve hls uri '{reference}': {err}")))
}

fn rewrite_tag_uri_attr(
    line: &str,
    base: &Url,
    proxy_base_url: &str,
    token: &str,
    signer: &HmacSigner,
) -> VesselResult<String> {
    let Some(uri_pos) = line.find("URI=\"") else {
        return Ok(line.to_owned());
    };
    let value_start = uri_pos + 5;
    let rest = &line[value_start..];
    let Some(rel_end) = rest.find('"') else {
        return Ok(line.to_owned());
    };
    let raw_uri = &rest[..rel_end];
    if raw_uri.is_empty() {
        return Ok(line.to_owned());
    }

    let resolved = resolve_url(base, raw_uri)?;
    let signed = build_signed_seg_url(proxy_base_url, token, signer, resolved.as_str());

    let mut updated = String::with_capacity(line.len() + signed.len());
    updated.push_str(&line[..value_start]);
    updated.push_str(&signed);
    updated.push_str(&rest[rel_end..]);
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::{decode_signed_seg_url, is_hls_playlist, rewrite_m3u8_playlist};
    use crate::HmacSigner;

    #[test]
    fn detects_hls_by_extension_or_content_type() {
        assert!(is_hls_playlist(
            "https://cdn.anthology.example/master.m3u8?token=abc",
            None
        ));
        assert!(is_hls_playlist(
            "https://cdn.anthology.example/stream",
            Some("application/vnd.apple.mpegurl; charset=utf-8")
        ));
        assert!(!is_hls_playlist(
            "https://cdn.anthology.example/video.mp4",
            Some("video/mp4")
        ));
    }

    #[test]
    fn rewrites_tags_and_segments_with_valid_hmac_signatures() {
        let signer = HmacSigner::from_key([9u8; 32]);
        let playlist = "\
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"Turkish\",URI=\"../subs/tr.m3u8\"
#EXT-X-KEY:METHOD=AES-128,URI=\"keys/enc.key\"
#EXT-X-MAP:URI=\"init.mp4\"
#EXTINF:6.000,
seg0.ts
";
        let rewritten = rewrite_m3u8_playlist(
            playlist,
            "https://cdn.anthology.example/hls/720p/index.m3u8",
            "http://127.0.0.1:41234",
            "tok123",
            &signer,
        )
        .expect("rewrite m3u8");

        let lines: Vec<&str> = rewritten.lines().collect();
        assert_eq!(lines[0], "#EXTM3U");
        assert!(lines[2].contains("http://127.0.0.1:41234/s/tok123/seg?u="));
        assert!(
            lines[3].starts_with(
                "#EXT-X-KEY:METHOD=AES-128,URI=\"http://127.0.0.1:41234/s/tok123/seg?u="
            )
        );
        assert!(lines[4].starts_with("#EXT-X-MAP:URI=\"http://127.0.0.1:41234/s/tok123/seg?u="));
        assert!(lines[6].starts_with("http://127.0.0.1:41234/s/tok123/seg?u="));

        // Verify the rewritten segment URL decodes back to the exact resolved upstream URL.
        let query = lines[6].split('?').nth(1).expect("query string present");
        let mut u_val = "";
        let mut sig_val = "";
        for pair in query.split('&') {
            if let Some(v) = pair.strip_prefix("u=") {
                u_val = v;
            } else if let Some(v) = pair.strip_prefix("sig=") {
                sig_val = v;
            }
        }
        assert_eq!(
            decode_signed_seg_url(&signer, "tok123", u_val, sig_val).as_deref(),
            Some("https://cdn.anthology.example/hls/720p/seg0.ts")
        );
    }
}
