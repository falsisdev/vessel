//! Streaming M3U and M3U-Plus playlist parser for IPTV live channels.

use std::collections::HashMap;

use crate::IptvChannel;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

fn fnv1a_64(input: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn stable_m3u_stream_id(stream_url: &str, seen_urls: &mut HashMap<String, usize>) -> String {
    let count = seen_urls.entry(stream_url.to_string()).or_insert(0);
    let idx = *count;
    *count += 1;
    let hash = fnv1a_64(stream_url);
    if idx == 0 {
        format!("m3u-{hash:016x}")
    } else {
        format!("m3u-{hash:016x}-{idx}")
    }
}

#[derive(Debug, Default)]
struct ExtInfMeta {
    name: String,
    tvg_id: Option<String>,
    tvg_logo: Option<String>,
    group_title: Option<String>,
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Splits `#EXTINF:<duration> attr="val" ...,Channel Name` at the comma outside quotes.
fn split_extinf_payload(rest: &str) -> (&str, &str) {
    let mut in_quote: Option<char> = None;
    for (idx, ch) in rest.char_indices() {
        match ch {
            '"' | '\'' => match in_quote {
                Some(q) if q == ch => in_quote = None,
                None => in_quote = Some(ch),
                _ => {}
            },
            ',' if in_quote.is_none() => {
                return (&rest[..idx], rest[idx + 1..].trim());
            }
            _ => {}
        }
    }
    (rest, "")
}

/// Parses `key="value"` / `key='value'` / `key=value` pairs inside the `#EXTINF` header.
fn parse_extinf(line: &str) -> ExtInfMeta {
    let body = line.trim_start_matches("#EXTINF:").trim();
    let (header_part, trailing_name) = split_extinf_payload(body);

    let mut meta = ExtInfMeta::default();
    let mut tvg_name: Option<String> = None;

    let bytes = header_part.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // Skip duration or unkeyed token.
            continue;
        }
        let key = header_part[key_start..i].trim().to_ascii_lowercase();
        i += 1; // consume '='

        let value = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let val_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let val = &header_part[val_start..i];
            if i < bytes.len() {
                i += 1;
            }
            val
        } else {
            let val_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            &header_part[val_start..i]
        };

        match key.as_str() {
            "tvg-id" => meta.tvg_id = non_empty(value),
            "tvg-name" => tvg_name = non_empty(value),
            "tvg-logo" => meta.tvg_logo = non_empty(value),
            "group-title" => meta.group_title = non_empty(value),
            _ => {}
        }
    }

    meta.name = non_empty(trailing_name)
        .or(tvg_name)
        .or_else(|| meta.tvg_id.clone())
        .unwrap_or_else(|| "Untitled Channel".to_string());

    meta
}

/// Parses an M3U / M3U-Plus document into normalized [`IptvChannel`] records.
///
/// Supports `#EXTINF` attributes (`tvg-id`, `tvg-name`, `tvg-logo`, `group-title`),
/// `#EXTGRP:<group>`, and `#EXTVLCOPT:http-user-agent=<ua>`.
pub fn parse_m3u(content: &str) -> Vec<IptvChannel> {
    let mut channels = Vec::new();
    let mut seen_urls: HashMap<String, usize> = HashMap::new();

    let mut pending_meta: Option<ExtInfMeta> = None;
    let mut pending_group: Option<String> = None;
    let mut pending_ua: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }

        if line.starts_with('#') {
            if line.starts_with("#EXTINF:") {
                pending_meta = Some(parse_extinf(line));
            } else if let Some(group) = line.strip_prefix("#EXTGRP:") {
                if let Some(clean) = non_empty(group) {
                    pending_group = Some(clean);
                }
            } else if let Some(opt) = line.strip_prefix("#EXTVLCOPT:")
                && let Some((k, v)) = opt.split_once('=')
                && k.trim().eq_ignore_ascii_case("http-user-agent")
            {
                pending_ua = non_empty(v);
            }
            continue;
        }

        let meta = pending_meta.take().unwrap_or_else(|| ExtInfMeta {
            name: line.to_string(),
            ..Default::default()
        });
        let group_title = meta
            .group_title
            .or_else(|| pending_group.take())
            .unwrap_or_else(|| "Uncategorized".to_string());
        let user_agent = pending_ua.take();
        let stream_id = stable_m3u_stream_id(line, &mut seen_urls);

        channels.push(IptvChannel {
            stream_id,
            name: meta.name,
            group_title,
            logo_url: meta.tvg_logo,
            stream_url: line.to_string(),
            tvg_id: meta.tvg_id,
            user_agent,
        });
    }

    channels
}

#[cfg(test)]
mod tests {
    use super::parse_m3u;

    #[test]
    fn parses_m3u_plus_attributes_extgrp_and_user_agent() {
        let m3u = "\u{feff}#EXTM3U url-tvg=\"https://epg.anthology.example/guide.xml\"\n\
                   #EXTINF:-1 tvg-id=\"news.one\" tvg-name=\"News One\" tvg-logo=\"https://cdn.anthology.example/n1.png\" group-title=\"News, World\",News One HD\n\
                   #EXTVLCOPT:http-user-agent=VesselPlayer/2.0\n\
                   https://iptv.anthology.example/live/news1.m3u8\n\
                   #EXTINF:0 tvg-id=\"\" group-title=\"\",Sports Arena\n\
                   #EXTGRP:Sports\n\
                   http://iptv.anthology.example:8080/live/sports/101.ts\n";

        let channels = parse_m3u(m3u);
        assert_eq!(channels.len(), 2);

        assert_eq!(channels[0].name, "News One HD");
        assert_eq!(channels[0].group_title, "News, World");
        assert_eq!(channels[0].tvg_id.as_deref(), Some("news.one"));
        assert_eq!(
            channels[0].logo_url.as_deref(),
            Some("https://cdn.anthology.example/n1.png")
        );
        assert_eq!(channels[0].user_agent.as_deref(), Some("VesselPlayer/2.0"));
        assert!(channels[0].stream_id.starts_with("m3u-"));

        assert_eq!(channels[1].name, "Sports Arena");
        assert_eq!(channels[1].group_title, "Sports");
        assert_eq!(channels[1].tvg_id, None);
        assert_eq!(channels[1].user_agent, None);
        assert_ne!(channels[0].stream_id, channels[1].stream_id);
    }

    #[test]
    fn generates_deterministic_stream_ids_across_syncs() {
        let m3u = "#EXTM3U\n#EXTINF:-1,Ch A\nhttps://iptv.anthology.example/a.ts\n";
        let first = parse_m3u(m3u);
        let second = parse_m3u(m3u);
        assert_eq!(first[0].stream_id, second[0].stream_id);
    }
}
