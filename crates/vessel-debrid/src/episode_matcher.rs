//! Smart video file & season-pack episode selection for multi-file torrents.

use vessel_domain::TorrentTarget;

/// Normalized file descriptor returned by Debrid torrent info APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFileCandidate {
    pub id: String,
    pub index: usize,
    pub path: String,
    pub size_bytes: u64,
}

const VIDEO_EXTENSIONS: &[&str] = &[".mkv", ".mp4", ".avi", ".mov", ".webm", ".m4v", ".ts"];
const JUNK_TOKENS: &[&str] = &[
    "sample",
    "trailer",
    "featurette",
    "deleted scene",
    "deleted.scene",
    "extras",
    "bonus",
    "ncop",
    "nced",
];

/// Returns `true` if `path` has a recognized video container extension.
pub fn is_video_file(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    VIDEO_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

fn is_junk_video(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    JUNK_TOKENS.iter().any(|tok| lower.contains(tok))
}

fn file_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Extracts `(season, episode)` or episode-only tokens from a filename/path.
fn score_episode_match(path: &str, target_season: Option<u32>, target_ep: u32) -> i64 {
    let lower = path.to_ascii_lowercase();
    let mut score: i64 = 0;

    if is_junk_video(&lower) {
        score -= 5000;
    }

    let ep_2 = format!("{target_ep:02}");
    let ep_str = target_ep.to_string();

    if let Some(season) = target_season {
        let s_2 = format!("{season:02}");
        let s_str = season.to_string();

        let exact_patterns = [
            format!("s{s_2}e{ep_2}"),
            format!("s{s_str}e{ep_2}"),
            format!("s{s_2}e{ep_str}"),
            format!("s{s_str}e{ep_str}"),
            format!("{s_2}x{ep_2}"),
            format!("{s_str}x{ep_2}"),
            format!("{s_str}x{ep_str}"),
        ];
        if exact_patterns.iter().any(|pat| has_token(&lower, pat)) {
            score += 2000;
        }

        // Penalize explicit mismatching seasons like S02E03 when targeting season 1.
        if let Some(found_season) = detect_explicit_season(&lower)
            && found_season != season
        {
            score -= 2500;
        }
    }

    let ep_only_patterns = [
        format!("e{ep_2}"),
        format!("ep{ep_2}"),
        format!("ep.{ep_2}"),
        format!("ep {ep_2}"),
        format!("episode {ep_2}"),
        format!("episode {ep_str}"),
        format!(" - {ep_2} "),
        format!(" - {ep_2}["),
        format!(" - {ep_2}("),
        format!(" - {ep_2}."),
        format!("[{ep_2}]"),
        format!("({ep_2})"),
        format!(".{ep_2}."),
    ];
    if ep_only_patterns.iter().any(|pat| has_token(&lower, pat)) {
        score += 800;
    }

    // Prefer exact single-digit padded boundary if episode < 10.
    if target_ep < 10 && !lower.contains(&ep_2) {
        let bare_ep = format!("e{ep_str}");
        if has_token(&lower, &bare_ep) {
            score += 600;
        }
    }

    score
}

fn has_token(haystack: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        let before_ok = abs == 0
            || !haystack[..abs]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric());
        let after_idx = abs + needle.len();
        let after_ok = after_idx >= haystack.len()
            || !haystack[after_idx..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

fn detect_explicit_season(lower: &str) -> Option<u32> {
    let bytes = lower.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b's' && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1
                && j < bytes.len()
                && bytes[j] == b'e'
                && let Ok(s) = lower[i + 1..j].parse::<u32>()
            {
                return Some(s);
            }
        }
    }
    None
}

/// Selects the best matching video file from a torrent's file list.
pub fn select_best_file<'a>(
    files: &'a [TorrentFileCandidate],
    target: &TorrentTarget,
) -> Option<&'a TorrentFileCandidate> {
    let video_files: Vec<&TorrentFileCandidate> =
        files.iter().filter(|f| is_video_file(&f.path)).collect();
    if video_files.is_empty() {
        return files.iter().max_by_key(|f| f.size_bytes);
    }

    // 1. Explicit file_index from Torrentio / Stremio `fileIdx`.
    if let Some(wanted_idx) = target.file_index
        && let Some(found) = video_files
            .iter()
            .copied()
            .find(|f| f.index == wanted_idx as usize)
    {
        return Some(found);
    }

    // 2. Explicit filename match.
    if let Some(wanted_name) = target.filename.as_deref().map(str::trim).filter(|s| !s.is_empty())
    {
        let wanted_base = file_basename(wanted_name).to_ascii_lowercase();
        if let Some(found) = video_files
            .iter()
            .copied()
            .find(|f| file_basename(&f.path).to_ascii_lowercase() == wanted_base)
        {
            return Some(found);
        }
    }

    // 3. Episode / season pack scoring.
    if let Some(ep) = target.episode {
        return video_files.into_iter().max_by(|a, b| {
            let sa = score_episode_match(&a.path, target.season, ep);
            let sb = score_episode_match(&b.path, target.season, ep);
            sa.cmp(&sb).then_with(|| a.size_bytes.cmp(&b.size_bytes))
        });
    }

    // 4. Movie fallback: non-junk largest video file.
    video_files
        .into_iter()
        .max_by_key(|f| {
            let penalty = if is_junk_video(&f.path) { 0u64 } else { 1u64 };
            (penalty, f.size_bytes)
        })
}

#[cfg(test)]
mod tests {
    use vessel_domain::TorrentTarget;

    use super::{TorrentFileCandidate, select_best_file};

    fn sample_pack() -> Vec<TorrentFileCandidate> {
        vec![
            TorrentFileCandidate {
                id: "1".into(),
                index: 0,
                path: "Show.S01/Show.S01E01.1080p.mkv".into(),
                size_bytes: 2_000_000_000,
            },
            TorrentFileCandidate {
                id: "2".into(),
                index: 1,
                path: "Show.S01/Show.S01E03.1080p.mkv".into(),
                size_bytes: 2_100_000_000,
            },
            TorrentFileCandidate {
                id: "3".into(),
                index: 2,
                path: "Show.S01/Sample/sample-show.mkv".into(),
                size_bytes: 50_000_000,
            },
            TorrentFileCandidate {
                id: "4".into(),
                index: 3,
                path: "Show.S01/readme.txt".into(),
                size_bytes: 1_024,
            },
        ]
    }

    #[test]
    fn picks_matching_episode_and_ignores_sample_and_txt() {
        let files = sample_pack();
        let target = TorrentTarget {
            season: Some(1),
            episode: Some(3),
            ..Default::default()
        };
        let picked = select_best_file(&files, &target).expect("picked episode");
        assert_eq!(picked.id, "2");
    }

    #[test]
    fn honors_explicit_file_index_over_heuristics() {
        let files = sample_pack();
        let target = TorrentTarget {
            file_index: Some(0),
            season: Some(1),
            episode: Some(3),
            ..Default::default()
        };
        let picked = select_best_file(&files, &target).expect("picked explicit index");
        assert_eq!(picked.id, "1");
    }

    #[test]
    fn picks_largest_non_junk_video_for_movie() {
        let files = vec![
            TorrentFileCandidate {
                id: "1".into(),
                index: 0,
                path: "Movie.2026/sample.mkv".into(),
                size_bytes: 100_000_000,
            },
            TorrentFileCandidate {
                id: "2".into(),
                index: 1,
                path: "Movie.2026/Movie.2026.2160p.REMUX.mkv".into(),
                size_bytes: 45_000_000_000,
            },
        ];
        let picked = select_best_file(&files, &TorrentTarget::default()).expect("picked movie");
        assert_eq!(picked.id, "2");
    }
}
