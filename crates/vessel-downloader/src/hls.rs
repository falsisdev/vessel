//! Parallel HLS offline downloader (4 workers + pure-Rust `AES-128-CBC` decryption + `local.m3u8`).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use aes::Aes128;
use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;
use vessel_domain::{
    DownloadProgressSnapshot, DownloadStatus, EngineEvent, EventSink, VesselError, VesselResult,
};
use vessel_storage::{SqliteStorage, StoredDownload, StoredDownloadSegment};

use crate::{apply_headers, bundle_subtitles};

/// Bounded worker count per active HLS download task (`PLAN.md` M5.3).
pub const HLS_SEGMENT_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterMediaTrack {
    pub media_type: String,
    pub group_id: String,
    pub name: String,
    pub language: Option<String>,
    pub is_default: bool,
    pub autoselect: bool,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterPlaylistSelection {
    pub variant_url: String,
    pub bandwidth: u64,
    pub media_tracks: Vec<MasterMediaTrack>,
}

type Aes128CbcDec = cbc::Decryptor<Aes128>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HlsKeySpec {
    None,
    Aes128 {
        key_url: String,
        iv: Option<[u8; 16]>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHlsSegment {
    pub seq: u32,
    pub media_sequence: u64,
    pub duration: f64,
    pub url: String,
    pub file_name: String,
    pub key: HlsKeySpec,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMediaPlaylist {
    pub target_duration: u64,
    pub map_url: Option<String>,
    pub segments: Vec<ParsedHlsSegment>,
}

fn parse_attr_list(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        let k_start = i;
        while i < bytes.len() && bytes[i] != b'=' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key = raw[k_start..i].trim().to_ascii_uppercase();
        i += 1; // '='
        let val = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let v_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let v = &raw[v_start..i];
            if i < bytes.len() {
                i += 1;
            }
            v
        } else {
            let v_start = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            raw[v_start..i].trim()
        };
        out.insert(key, val.to_string());
    }
    out
}

fn resolve_url(base: &Url, reference: &str) -> VesselResult<String> {
    base.join(reference.trim())
        .map(String::from)
        .map_err(|err| VesselError::InvalidInput(format!("invalid HLS URI '{reference}': {err}")))
}

fn parse_hex_iv(raw: &str) -> VesselResult<[u8; 16]> {
    let hex = raw
        .trim()
        .strip_prefix("0x")
        .or_else(|| raw.trim().strip_prefix("0X"))
        .unwrap_or(raw.trim());
    if hex.len() != 32 || !hex.is_ascii() {
        return Err(VesselError::InvalidInput(format!(
            "invalid AES-128 IV length in '{raw}'"
        )));
    }
    let mut iv = [0u8; 16];
    let (pairs, rem) = hex.as_bytes().as_chunks::<2>();
    if !rem.is_empty() {
        return Err(VesselError::InvalidInput("odd AES-128 IV length".into()));
    }
    for (slot, pair) in iv.iter_mut().zip(pairs.iter()) {
        let s = std::str::from_utf8(pair)
            .map_err(|err| VesselError::InvalidInput(format!("invalid IV UTF-8: {err}")))?;
        *slot = u8::from_str_radix(s, 16).map_err(|err| {
            VesselError::InvalidInput(format!("invalid IV hex byte '{s}': {err}"))
        })?;
    }
    Ok(iv)
}

/// Selects the highest-bandwidth stream URI and any matching `AUDIO`/`SUBTITLES` media groups.
pub fn parse_master_playlist(
    playlist: &str,
    base_url: &Url,
) -> VesselResult<Option<MasterPlaylistSelection>> {
    let mut best: Option<(u64, String, Option<String>, Option<String>)> = None;
    let mut pending: Option<(u64, Option<String>, Option<String>)> = None;
    let mut all_media: Vec<MasterMediaTrack> = Vec::new();

    for raw in playlist.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXT-X-MEDIA:") {
            let attrs = parse_attr_list(rest);
            let media_type = attrs
                .get("TYPE")
                .map(|s| s.to_ascii_uppercase())
                .unwrap_or_default();
            if (media_type == "AUDIO" || media_type == "SUBTITLES")
                && let Some(raw_uri) = attrs.get("URI")
                && !raw_uri.trim().is_empty()
                && let Ok(uri) = resolve_url(base_url, raw_uri)
            {
                all_media.push(MasterMediaTrack {
                    media_type,
                    group_id: attrs.get("GROUP-ID").cloned().unwrap_or_default(),
                    name: attrs
                        .get("NAME")
                        .cloned()
                        .unwrap_or_else(|| "Track".to_string()),
                    language: attrs.get("LANGUAGE").cloned(),
                    is_default: attrs
                        .get("DEFAULT")
                        .is_some_and(|v| v.eq_ignore_ascii_case("YES")),
                    autoselect: attrs
                        .get("AUTOSELECT")
                        .is_some_and(|v| v.eq_ignore_ascii_case("YES")),
                    uri,
                });
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let attrs = parse_attr_list(rest);
            let bw = attrs
                .get("BANDWIDTH")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            pending = Some((
                bw,
                attrs.get("AUDIO").cloned(),
                attrs.get("SUBTITLES").cloned(),
            ));
            continue;
        }
        if let Some((bw, audio_grp, sub_grp)) = pending.take()
            && !line.starts_with('#')
        {
            let resolved = resolve_url(base_url, line)?;
            if best.as_ref().is_none_or(|(b, _, _, _)| bw > *b) {
                best = Some((bw, resolved, audio_grp, sub_grp));
            }
        }
    }

    let Some((bandwidth, variant_url, audio_grp, sub_grp)) = best else {
        return Ok(None);
    };

    let media_tracks = all_media
        .into_iter()
        .filter(|m| {
            if m.media_type == "AUDIO" {
                audio_grp
                    .as_ref()
                    .is_none_or(|g| g.is_empty() || m.group_id == *g)
            } else {
                sub_grp
                    .as_ref()
                    .is_none_or(|g| g.is_empty() || m.group_id == *g)
            }
        })
        .collect();

    Ok(Some(MasterPlaylistSelection {
        variant_url,
        bandwidth,
        media_tracks,
    }))
}

/// Selects the highest-bandwidth stream URI from a master playlist, or `None` if already a media playlist.
pub fn select_highest_variant(playlist: &str, base_url: &Url) -> VesselResult<Option<String>> {
    Ok(parse_master_playlist(playlist, base_url)?.map(|sel| sel.variant_url))
}

/// Parses a resolved HLS media playlist including `#EXT-X-KEY` and `#EXT-X-MAP`.
pub fn parse_media_playlist(playlist: &str, base_url: &Url) -> VesselResult<ParsedMediaPlaylist> {
    let mut target_duration = 6u64;
    let mut base_media_seq = 0u64;
    let mut next_extinf: Option<f64> = None;
    let mut current_key = HlsKeySpec::None;
    let mut map_url = None;
    let mut segments = Vec::new();

    for raw in playlist.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(dur) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            if let Ok(parsed) = dur.trim().parse::<f64>() {
                target_duration = parsed.ceil().max(1.0) as u64;
            }
        } else if let Some(seq) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            if let Ok(parsed) = seq.trim().parse::<u64>() {
                base_media_seq = parsed;
            }
        } else if let Some(rest) = line.strip_prefix("#EXT-X-KEY:") {
            let attrs = parse_attr_list(rest);
            let method = attrs
                .get("METHOD")
                .map(|s| s.to_ascii_uppercase())
                .unwrap_or_else(|| "NONE".to_string());
            match method.as_str() {
                "NONE" => current_key = HlsKeySpec::None,
                "AES-128" => {
                    let uri = attrs.get("URI").ok_or_else(|| {
                        VesselError::InvalidInput("HLS AES-128 key tag missing URI".into())
                    })?;
                    let key_url = resolve_url(base_url, uri)?;
                    let iv = match attrs.get("IV") {
                        Some(raw_iv) => Some(parse_hex_iv(raw_iv)?),
                        None => None,
                    };
                    current_key = HlsKeySpec::Aes128 { key_url, iv };
                }
                other => {
                    return Err(VesselError::InvalidInput(format!(
                        "unsupported HLS encryption method '{other}'"
                    )));
                }
            }
        } else if let Some(rest) = line.strip_prefix("#EXT-X-MAP:") {
            let attrs = parse_attr_list(rest);
            if let Some(uri) = attrs.get("URI") {
                map_url = Some(resolve_url(base_url, uri)?);
            }
        } else if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let dur_part = rest.split(',').next().unwrap_or("0").trim();
            next_extinf = Some(dur_part.parse::<f64>().unwrap_or(0.0).max(0.0));
        } else if !line.starts_with('#') {
            let seq = u32::try_from(segments.len()).map_err(|_| {
                VesselError::InvalidInput("HLS playlist exceeds u32 segments".into())
            })?;
            let media_sequence = base_media_seq.saturating_add(u64::from(seq));
            let ext = if map_url.is_some() { "m4s" } else { "ts" };
            let file_name = format!("seg_{seq:05}.{ext}");
            segments.push(ParsedHlsSegment {
                seq,
                media_sequence,
                duration: next_extinf.take().unwrap_or(target_duration as f64),
                url: resolve_url(base_url, line)?,
                file_name,
                key: current_key.clone(),
            });
        }
    }

    if segments.is_empty() {
        return Err(VesselError::InvalidInput(
            "HLS media playlist contains no segments".into(),
        ));
    }

    Ok(ParsedMediaPlaylist {
        target_duration,
        map_url,
        segments,
    })
}

/// Decrypts an `AES-128-CBC` PKCS#7 encrypted HLS segment in memory.
pub fn decrypt_aes128_cbc_segment(
    ciphertext: &[u8],
    key: &[u8; 16],
    iv: &[u8; 16],
) -> VesselResult<Vec<u8>> {
    let mut buf = ciphertext.to_vec();
    let plain = Aes128CbcDec::new(key.into(), iv.into())
        .decrypt_padded::<Pkcs7>(&mut buf)
        .map_err(|err| {
            VesselError::InvalidInput(format!("AES-128 HLS segment decryption failed: {err}"))
        })?;
    let plain_len = plain.len();
    buf.truncate(plain_len);
    Ok(buf)
}

fn default_media_seq_iv(media_sequence: u64) -> [u8; 16] {
    u128::from(media_sequence).to_be_bytes()
}

async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> VesselResult<Vec<u8>> {
    let req = apply_headers(client.get(url), headers);
    let resp = tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = req.send() => res.map_err(|err| VesselError::Network(format!("HTTP GET '{url}' failed: {err}")))?,
    };
    if !resp.status().is_success() {
        return Err(VesselError::Network(format!(
            "HTTP GET '{url}' returned status {}",
            resp.status()
        )));
    }
    let bytes = tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = resp.bytes() => res.map_err(|err| VesselError::Network(format!("read response bytes from '{url}': {err}")))?,
    };
    Ok(bytes.to_vec())
}

async fn fetch_text(
    client: &reqwest::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> VesselResult<(Url, String)> {
    let req = apply_headers(client.get(url), headers);
    let resp = tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = req.send() => res.map_err(|err| VesselError::Network(format!("HTTP GET '{url}' failed: {err}")))?,
    };
    if !resp.status().is_success() {
        return Err(VesselError::Network(format!(
            "HTTP GET '{url}' returned status {}",
            resp.status()
        )));
    }
    let final_url = resp.url().clone();
    let text = tokio::select! {
        _ = cancel.cancelled() => return Err(VesselError::Cancelled),
        res = resp.text() => res.map_err(|err| VesselError::Network(format!("read playlist text from '{url}': {err}")))?,
    };
    Ok((final_url, text))
}

fn render_local_playlist(playlist: &ParsedMediaPlaylist, has_init_map: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(&mut out, "#EXTM3U");
    let _ = writeln!(&mut out, "#EXT-X-VERSION:3");
    let _ = writeln!(
        &mut out,
        "#EXT-X-TARGETDURATION:{}",
        playlist.target_duration
    );
    let _ = writeln!(&mut out, "#EXT-X-MEDIA-SEQUENCE:0");
    if has_init_map {
        let _ = writeln!(&mut out, "#EXT-X-MAP:URI=\"init.mp4\"");
    }
    for seg in &playlist.segments {
        let _ = writeln!(&mut out, "#EXTINF:{:.3},", seg.duration);
        let _ = writeln!(&mut out, "{}", seg.file_name);
    }
    let _ = writeln!(&mut out, "#EXT-X-ENDLIST");
    out
}

pub(crate) async fn run_hls_download(
    client: &reqwest::Client,
    storage: &Arc<SqliteStorage>,
    event_sink: &Arc<dyn EventSink>,
    record: &StoredDownload,
    cancel: &CancellationToken,
) -> VesselResult<PathBuf> {
    let local_dir = PathBuf::from(&record.local_dir);
    fs::create_dir_all(&local_dir)
        .await
        .map_err(|err| VesselError::Storage(format!("create HLS directory: {err}")))?;

    let (base_url, raw_playlist) =
        fetch_text(client, &record.source_url, &record.headers, cancel).await?;
    let (media_url, media_text, master_selection) =
        match parse_master_playlist(&raw_playlist, &base_url)? {
            Some(sel) => {
                let (m_url, m_txt) =
                    fetch_text(client, &sel.variant_url, &record.headers, cancel).await?;
                (m_url, m_txt, Some(sel))
            }
            None => (base_url, raw_playlist, None),
        };
    let playlist = parse_media_playlist(&media_text, &media_url)?;
    let total_segments = u32::try_from(playlist.segments.len()).unwrap_or(u32::MAX);

    let has_init_map = if let Some(ref map_url) = playlist.map_url {
        let init_path = local_dir.join("init.mp4");
        if fs::metadata(&init_path).await.is_err() {
            let init_bytes = fetch_bytes(client, map_url, &record.headers, cancel).await?;
            fs::write(&init_path, &init_bytes)
                .await
                .map_err(|err| VesselError::Storage(format!("write HLS init segment: {err}")))?;
        }
        true
    } else {
        false
    };

    // Fetch all unique AES-128 keys up front.
    let mut key_cache: HashMap<String, [u8; 16]> = HashMap::new();
    for seg in &playlist.segments {
        if let HlsKeySpec::Aes128 { key_url, .. } = &seg.key
            && !key_cache.contains_key(key_url)
        {
            let key_bytes = fetch_bytes(client, key_url, &record.headers, cancel).await?;
            let key_arr: [u8; 16] = key_bytes.as_slice().try_into().map_err(|_| {
                VesselError::InvalidInput(format!(
                    "HLS AES-128 key from '{key_url}' must be 16 bytes, got {}",
                    key_bytes.len()
                ))
            })?;
            key_cache.insert(key_url.clone(), key_arr);
        }
    }
    let key_cache = Arc::new(key_cache);

    let existing_segments = storage.list_download_segments(&record.id).await?;
    if existing_segments.is_empty() {
        let seeded: Vec<StoredDownloadSegment> = playlist
            .segments
            .iter()
            .map(|seg| StoredDownloadSegment {
                download_id: record.id.clone(),
                seq: seg.seq,
                url: seg.url.clone(),
                file_name: seg.file_name.clone(),
                completed: false,
            })
            .collect();
        storage.seed_download_segments(&record.id, seeded).await?;
    }

    let mut completed_map = HashMap::new();
    for seg in storage.list_download_segments(&record.id).await? {
        if seg.completed && fs::metadata(local_dir.join(&seg.file_name)).await.is_ok() {
            completed_map.insert(seg.seq, true);
        }
    }

    let initial_done = u32::try_from(completed_map.len()).unwrap_or(0);
    let mut initial_bytes = 0u64;
    for seg in &playlist.segments {
        if completed_map.contains_key(&seg.seq)
            && let Ok(meta) = fs::metadata(local_dir.join(&seg.file_name)).await
        {
            initial_bytes = initial_bytes.saturating_add(meta.len());
        }
    }

    storage
        .update_download_progress(
            &record.id,
            DownloadStatus::Downloading,
            initial_bytes,
            None,
            initial_done,
            total_segments,
            None,
            None,
        )
        .await?;
    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
        download_id: record.id.clone(),
        status: DownloadStatus::Downloading,
        bytes_downloaded: initial_bytes,
        total_bytes: None,
        segments_done: initial_done,
        segments_total: total_segments,
    }));

    let done_counter = Arc::new(AtomicU32::new(initial_done));
    let bytes_counter = Arc::new(AtomicU64::new(initial_bytes));
    let worker_sem = Arc::new(Semaphore::new(HLS_SEGMENT_CONCURRENCY));
    let headers = Arc::new(record.headers.clone());
    let mut join_set: JoinSet<VesselResult<()>> = JoinSet::new();

    for seg in playlist.segments.iter().cloned() {
        if completed_map.contains_key(&seg.seq) {
            continue;
        }
        let permit = tokio::select! {
            _ = cancel.cancelled() => break,
            res = Arc::clone(&worker_sem).acquire_owned() => {
                res.map_err(|_| VesselError::Cancelled)?
            }
        };
        let client = client.clone();
        let storage = Arc::clone(storage);
        let event_sink = Arc::clone(event_sink);
        let key_cache = Arc::clone(&key_cache);
        let headers = Arc::clone(&headers);
        let done_counter = Arc::clone(&done_counter);
        let bytes_counter = Arc::clone(&bytes_counter);
        let local_dir = local_dir.clone();
        let download_id = record.id.clone();
        let cancel = cancel.clone();

        join_set.spawn(async move {
            let _permit = permit;
            let raw_bytes = fetch_bytes(&client, &seg.url, &headers, &cancel).await?;
            let output_bytes = match &seg.key {
                HlsKeySpec::None => raw_bytes,
                HlsKeySpec::Aes128 { key_url, iv } => {
                    let key = key_cache.get(key_url).ok_or_else(|| {
                        VesselError::InvalidInput(format!("missing cached key for '{key_url}'"))
                    })?;
                    let effective_iv =
                        iv.unwrap_or_else(|| default_media_seq_iv(seg.media_sequence));
                    decrypt_aes128_cbc_segment(&raw_bytes, key, &effective_iv)?
                }
            };

            let part_path = local_dir.join(format!("{}.part", seg.file_name));
            let final_path = local_dir.join(&seg.file_name);
            fs::write(&part_path, &output_bytes)
                .await
                .map_err(|err| VesselError::Storage(format!("write segment {}: {err}", seg.seq)))?;
            fs::rename(&part_path, &final_path).await.map_err(|err| {
                VesselError::Storage(format!("rename segment {}: {err}", seg.seq))
            })?;

            storage
                .mark_segment_completed(&download_id, seg.seq)
                .await?;
            let done = done_counter.fetch_add(1, Ordering::AcqRel) + 1;
            let bytes = bytes_counter.fetch_add(
                u64::try_from(output_bytes.len()).unwrap_or(0),
                Ordering::AcqRel,
            ) + u64::try_from(output_bytes.len()).unwrap_or(0);

            storage
                .update_download_progress(
                    &download_id,
                    DownloadStatus::Downloading,
                    bytes,
                    None,
                    done,
                    total_segments,
                    None,
                    None,
                )
                .await?;
            event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                download_id,
                status: DownloadStatus::Downloading,
                bytes_downloaded: bytes,
                total_bytes: None,
                segments_done: done,
                segments_total: total_segments,
            }));
            Ok(())
        });
    }

    let mut first_error: Option<VesselError> = None;
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if first_error.is_none() {
                    first_error = Some(err);
                    cancel.cancel();
                }
            }
            Err(join_err) => {
                if first_error.is_none() {
                    first_error = Some(VesselError::Storage(format!(
                        "HLS segment worker panicked: {join_err}"
                    )));
                    cancel.cancel();
                }
            }
        }
    }

    if let Some(err) = first_error {
        let done = done_counter.load(Ordering::Acquire);
        let bytes = bytes_counter.load(Ordering::Acquire);
        if err == VesselError::Cancelled {
            storage
                .update_download_progress(
                    &record.id,
                    DownloadStatus::Paused,
                    bytes,
                    None,
                    done,
                    total_segments,
                    None,
                    None,
                )
                .await?;
            event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
                download_id: record.id.clone(),
                status: DownloadStatus::Paused,
                bytes_downloaded: bytes,
                total_bytes: None,
                segments_done: done,
                segments_total: total_segments,
            }));
        }
        return Err(err);
    }

    let mut downloaded_media_tracks: Vec<(MasterMediaTrack, String)> = Vec::new();
    if let Some(ref master) = master_selection
        && !master.media_tracks.is_empty()
    {
        for (idx, track) in master.media_tracks.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(VesselError::Cancelled);
            }
            let prefix = if track.media_type == "AUDIO" {
                format!("audio_{idx}")
            } else {
                format!("sub_hls_{idx}")
            };
            let track_dir = local_dir.join(&prefix);
            if let Ok(rel_uri) = download_aux_hls_media(
                client,
                track,
                &record.headers,
                &track_dir,
                &prefix,
                &worker_sem,
                cancel,
            )
            .await
            {
                downloaded_media_tracks.push((track.clone(), rel_uri));
            }
        }
    }

    let local_m3u8_path = local_dir.join("local.m3u8");
    let video_playlist_content = render_local_playlist(&playlist, has_init_map);
    if downloaded_media_tracks.is_empty() {
        fs::write(&local_m3u8_path, video_playlist_content.as_bytes())
            .await
            .map_err(|err| VesselError::Storage(format!("write local.m3u8: {err}")))?;
    } else {
        let video_m3u8_path = local_dir.join("video.m3u8");
        fs::write(&video_m3u8_path, video_playlist_content.as_bytes())
            .await
            .map_err(|err| VesselError::Storage(format!("write video.m3u8: {err}")))?;

        let master_content = render_local_master_playlist(
            master_selection
                .as_ref()
                .map(|s| s.bandwidth)
                .unwrap_or(2_500_000),
            &downloaded_media_tracks,
        );
        fs::write(&local_m3u8_path, master_content.as_bytes())
            .await
            .map_err(|err| VesselError::Storage(format!("write local.m3u8 master: {err}")))?;
    }

    bundle_subtitles(client, &record.subtitles, &record.headers, &local_dir).await?;

    let final_bytes = bytes_counter.load(Ordering::Acquire);
    let final_path_str = local_m3u8_path.to_string_lossy().into_owned();
    storage
        .update_download_progress(
            &record.id,
            DownloadStatus::Completed,
            final_bytes,
            Some(final_bytes),
            total_segments,
            total_segments,
            Some(&final_path_str),
            None,
        )
        .await?;
    event_sink.emit(EngineEvent::DownloadProgress(DownloadProgressSnapshot {
        download_id: record.id.clone(),
        status: DownloadStatus::Completed,
        bytes_downloaded: final_bytes,
        total_bytes: Some(final_bytes),
        segments_done: total_segments,
        segments_total: total_segments,
    }));

    Ok(local_m3u8_path)
}

/// Optional desktop helper invoking `ffmpeg -c copy` to remux `local.m3u8` into `media.mp4`.
pub async fn remux_local_hls_to_mp4(local_dir: &Path) -> VesselResult<PathBuf> {
    let input = local_dir.join("local.m3u8");
    let output = local_dir.join("media.mp4");
    let status = Command::new("ffmpeg")
        .arg("-y")
        .args(["-allowed_extensions", "ALL"])
        .arg("-i")
        .arg(&input)
        .args(["-c", "copy"])
        .arg(&output)
        .status()
        .await
        .map_err(|err| VesselError::Storage(format!("failed spawning ffmpeg remux: {err}")))?;

    if !status.success() {
        return Err(VesselError::Storage(format!(
            "ffmpeg remux exited with status {status}"
        )));
    }
    Ok(output)
}

async fn download_aux_hls_media(
    client: &reqwest::Client,
    track: &MasterMediaTrack,
    headers: &BTreeMap<String, String>,
    track_dir: &Path,
    prefix: &str,
    worker_sem: &Arc<Semaphore>,
    cancel: &CancellationToken,
) -> VesselResult<String> {
    fs::create_dir_all(track_dir)
        .await
        .map_err(|err| VesselError::Storage(format!("create HLS aux dir: {err}")))?;

    let (base_url, text) = fetch_text(client, &track.uri, headers, cancel).await?;
    let parsed = parse_media_playlist(&text, &base_url)?;
    let has_init_map = if let Some(ref map_url) = parsed.map_url {
        let init_bytes = fetch_bytes(client, map_url, headers, cancel).await?;
        fs::write(track_dir.join("init.mp4"), &init_bytes)
            .await
            .map_err(|err| VesselError::Storage(format!("write aux init.mp4: {err}")))?;
        true
    } else {
        false
    };

    let mut key_cache: HashMap<String, [u8; 16]> = HashMap::new();
    for seg in &parsed.segments {
        if let HlsKeySpec::Aes128 { key_url, .. } = &seg.key
            && !key_cache.contains_key(key_url)
        {
            let key_bytes = fetch_bytes(client, key_url, headers, cancel).await?;
            let key_arr: [u8; 16] = key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| VesselError::InvalidInput("invalid aux AES-128 key".into()))?;
            key_cache.insert(key_url.clone(), key_arr);
        }
    }
    let key_cache = Arc::new(key_cache);
    let headers = Arc::new(headers.clone());
    let mut join_set: JoinSet<VesselResult<()>> = JoinSet::new();

    for seg in parsed.segments.iter().cloned() {
        let permit = Arc::clone(worker_sem)
            .acquire_owned()
            .await
            .map_err(|_| VesselError::Cancelled)?;
        let client = client.clone();
        let headers = Arc::clone(&headers);
        let key_cache = Arc::clone(&key_cache);
        let track_dir = track_dir.to_path_buf();
        let cancel = cancel.clone();

        join_set.spawn(async move {
            let _permit = permit;
            let raw = fetch_bytes(&client, &seg.url, &headers, &cancel).await?;
            let out = match &seg.key {
                HlsKeySpec::None => raw,
                HlsKeySpec::Aes128 { key_url, iv } => {
                    let key = key_cache
                        .get(key_url)
                        .ok_or_else(|| VesselError::InvalidInput("missing aux HLS key".into()))?;
                    let eff_iv = iv.unwrap_or_else(|| default_media_seq_iv(seg.media_sequence));
                    decrypt_aes128_cbc_segment(&raw, key, &eff_iv)?
                }
            };
            fs::write(track_dir.join(&seg.file_name), &out)
                .await
                .map_err(|err| VesselError::Storage(format!("write aux seg: {err}")))?;
            Ok(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(err),
            Err(err) => {
                return Err(VesselError::Storage(format!("aux HLS task failed: {err}")));
            }
        }
    }

    let aux_playlist = render_local_playlist(&parsed, has_init_map);
    fs::write(track_dir.join("local.m3u8"), aux_playlist.as_bytes())
        .await
        .map_err(|err| VesselError::Storage(format!("write aux local.m3u8: {err}")))?;
    Ok(format!("{prefix}/local.m3u8"))
}

fn render_local_master_playlist(
    bandwidth: u64,
    media_tracks: &[(MasterMediaTrack, String)],
) -> String {
    let mut out = String::new();
    let _ = writeln!(&mut out, "#EXTM3U");
    let _ = writeln!(&mut out, "#EXT-X-VERSION:4");

    let mut audio_group: Option<String> = None;
    let mut sub_group: Option<String> = None;

    for (track, rel_uri) in media_tracks {
        let gid = if track.group_id.is_empty() {
            if track.media_type == "AUDIO" {
                "audio".to_string()
            } else {
                "subs".to_string()
            }
        } else {
            track.group_id.clone()
        };
        if track.media_type == "AUDIO" && audio_group.is_none() {
            audio_group = Some(gid.clone());
        }
        if track.media_type == "SUBTITLES" && sub_group.is_none() {
            sub_group = Some(gid.clone());
        }
        let mut attrs = format!(
            "TYPE={},GROUP-ID=\"{}\",NAME=\"{}\"",
            track.media_type,
            gid.replace('"', ""),
            track.name.replace('"', "")
        );
        if let Some(ref lang) = track.language {
            let _ = write!(&mut attrs, ",LANGUAGE=\"{}\"", lang.replace('"', ""));
        }
        let _ = write!(
            &mut attrs,
            ",DEFAULT={},AUTOSELECT={},URI=\"{}\"",
            if track.is_default { "YES" } else { "NO" },
            if track.autoselect { "YES" } else { "NO" },
            rel_uri
        );
        let _ = writeln!(&mut out, "#EXT-X-MEDIA:{attrs}");
    }

    let mut stream_inf = format!("#EXT-X-STREAM-INF:BANDWIDTH={}", bandwidth.max(1));
    if let Some(ref ag) = audio_group {
        let _ = write!(&mut stream_inf, ",AUDIO=\"{ag}\"");
    }
    if let Some(ref sg) = sub_group {
        let _ = write!(&mut stream_inf, ",SUBTITLES=\"{sg}\"");
    }
    let _ = writeln!(&mut out, "{stream_inf}");
    let _ = writeln!(&mut out, "video.m3u8");
    out
}
