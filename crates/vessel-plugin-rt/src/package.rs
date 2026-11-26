//! `.plugin.vessel` ZIP archive loading/packing, Dev Mode directory loading, and `oxc` import tracing.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use oxc::allocator::Allocator;
use oxc::parser::Parser;
use oxc::span::SourceType;
use vessel_domain::{PluginManifest, VesselError, VesselResult};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::module_loader::PluginBundle;
use crate::ts_strip::sha256_hex;

/// Maximum uncompressed bytes accepted from a `.plugin.vessel` ZIP archive (16 MiB).
pub const MAX_PACKAGE_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum individual source file size inside a plugin package (2 MiB).
pub const MAX_PACKAGE_ENTRY_BYTES: u64 = 2 * 1024 * 1024;

/// Decoded plugin package ready for runtime loading.
#[derive(Debug, Clone)]
pub struct LoadedPackage {
    pub manifest: PluginManifest,
    pub bundle: PluginBundle,
    pub icon_png: Option<Vec<u8>>,
    pub sha256: String,
}

/// Verifies `bytes` against an expected lowercase/uppercase 64-character hex SHA-256 digest.
pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> VesselResult<String> {
    let actual = sha256_hex(bytes);
    let expected_norm = expected_hex.trim().to_ascii_lowercase();
    if actual != expected_norm {
        return Err(VesselError::InvalidInput(format!(
            "SHA-256 mismatch: expected '{expected_norm}', got '{actual}'"
        )));
    }
    Ok(actual)
}

fn is_text_module_file(path: &str) -> bool {
    path.ends_with(".ts")
        || path.ends_with(".mts")
        || path.ends_with(".tsx")
        || path.ends_with(".js")
        || path.ends_with(".mjs")
        || path.ends_with(".cjs")
        || path.ends_with(".json")
}

fn read_limited<R: Read>(reader: R, max_bytes: u64, label: &str) -> VesselResult<Vec<u8>> {
    let mut limited = reader.take(max_bytes + 1);
    let mut buf = Vec::new();
    limited
        .read_to_end(&mut buf)
        .map_err(|err| VesselError::InvalidInput(format!("read '{label}': {err}")))?;
    if buf.len() as u64 > max_bytes {
        return Err(VesselError::InvalidInput(format!(
            "entry '{label}' exceeds maximum allowed size ({max_bytes} bytes)"
        )));
    }
    Ok(buf)
}

fn build_package_from_raw(
    raw_entries: BTreeMap<String, Vec<u8>>,
    sha256: String,
) -> VesselResult<LoadedPackage> {
    // Support both root-level `manifest.json` and single top-level wrapper folder (`anthology/manifest.json`).
    let stripped_entries = if raw_entries.contains_key("manifest.json") {
        raw_entries
    } else {
        let manifest_key = raw_entries
            .keys()
            .find(|path| path.ends_with("/manifest.json") && path.matches('/').count() == 1)
            .cloned()
            .ok_or_else(|| {
                VesselError::NotFound("manifest.json not found in plugin package".into())
            })?;
        let prefix = manifest_key
            .strip_suffix("manifest.json")
            .unwrap_or_default()
            .to_owned();
        raw_entries
            .into_iter()
            .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|rel| (rel.to_owned(), v)))
            .collect()
    };

    let manifest_bytes = stripped_entries
        .get("manifest.json")
        .ok_or_else(|| VesselError::NotFound("manifest.json not found in plugin package".into()))?;
    let manifest: PluginManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|err| VesselError::InvalidInput(format!("invalid manifest.json: {err}")))?;
    manifest.validate()?;

    let icon_png = stripped_entries.get("icon.png").cloned();

    let mut module_sources = BTreeMap::new();
    for (path, bytes) in stripped_entries {
        if path == "manifest.json" || !is_text_module_file(&path) {
            continue;
        }
        let text = String::from_utf8(bytes).map_err(|err| {
            VesselError::InvalidInput(format!("non-UTF8 module file '{path}': {err}"))
        })?;
        module_sources.insert(path, text);
    }

    let bundle = PluginBundle::new(module_sources)?;
    // Ensure the declared entrypoint exists inside the loaded bundle.
    bundle.resolve_entry(&manifest.entry)?;

    Ok(LoadedPackage {
        manifest,
        bundle,
        icon_png,
        sha256,
    })
}

/// Loads and validates a `.plugin.vessel` ZIP archive with optional SHA-256 verification.
pub fn load_plugin_package(
    zip_bytes: &[u8],
    expected_sha256: Option<&str>,
) -> VesselResult<LoadedPackage> {
    let sha256 = match expected_sha256 {
        Some(expected) => verify_sha256(zip_bytes, expected)?,
        None => sha256_hex(zip_bytes),
    };

    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|err| VesselError::InvalidInput(format!("invalid .plugin.vessel zip: {err}")))?;

    let mut total_uncompressed: u64 = 0;
    let mut raw_entries = BTreeMap::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|err| {
            VesselError::InvalidInput(format!("corrupt zip entry #{index}: {err}"))
        })?;
        if entry.is_dir() {
            continue;
        }

        let Some(rel_path) = entry.enclosed_name() else {
            return Err(VesselError::InvalidInput(
                "unsafe path traversal entry in .plugin.vessel archive".into(),
            ));
        };
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");

        if entry.size() > MAX_PACKAGE_ENTRY_BYTES {
            return Err(VesselError::InvalidInput(format!(
                "zip entry '{rel_str}' exceeds max size"
            )));
        }

        let bytes = read_limited(entry, MAX_PACKAGE_ENTRY_BYTES, &rel_str)?;
        total_uncompressed = total_uncompressed.saturating_add(bytes.len() as u64);
        if total_uncompressed > MAX_PACKAGE_UNCOMPRESSED_BYTES {
            return Err(VesselError::InvalidInput(
                "plugin package exceeds maximum uncompressed size limit (16 MiB)".into(),
            ));
        }

        raw_entries.insert(rel_str, bytes);
    }

    build_package_from_raw(raw_entries, sha256)
}

/// Builds a validated `LoadedPackage` from in-memory built-in manifest and module sources.
pub fn load_builtin_package(
    manifest_json: &str,
    files: BTreeMap<String, String>,
) -> VesselResult<LoadedPackage> {
    let mut raw_entries = BTreeMap::new();
    raw_entries.insert(
        "manifest.json".to_owned(),
        manifest_json.as_bytes().to_vec(),
    );
    let mut hasher_input = String::from(manifest_json);
    for (path, content) in files {
        let clean = path.trim_start_matches("./").to_owned();
        hasher_input.push_str(&clean);
        hasher_input.push_str(&content);
        raw_entries.insert(clean, content.into_bytes());
    }
    let sha256 = sha256_hex(hasher_input.as_bytes());
    build_package_from_raw(raw_entries, sha256)
}

/// Packs a validated `PluginManifest`, source files, and optional icon into `.plugin.vessel` ZIP bytes.
pub fn pack_plugin_package(
    manifest: &PluginManifest,
    files: &BTreeMap<String, String>,
    icon_png: Option<&[u8]>,
) -> VesselResult<Vec<u8>> {
    manifest.validate()?;
    let manifest_json = serde_json::to_vec_pretty(manifest)
        .map_err(|err| VesselError::InvalidInput(format!("serialize manifest.json: {err}")))?;

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        writer
            .start_file("manifest.json", options)
            .map_err(|err| VesselError::Storage(format!("zip manifest.json: {err}")))?;
        writer
            .write_all(&manifest_json)
            .map_err(|err| VesselError::Storage(format!("write zip manifest.json: {err}")))?;

        if let Some(icon) = icon_png {
            writer
                .start_file("icon.png", options)
                .map_err(|err| VesselError::Storage(format!("zip icon.png: {err}")))?;
            writer
                .write_all(icon)
                .map_err(|err| VesselError::Storage(format!("write zip icon.png: {err}")))?;
        }

        for (rel_path, content) in files {
            let clean = rel_path.trim_start_matches("./");
            if clean == "manifest.json" || clean == "icon.png" {
                continue;
            }
            writer
                .start_file(clean, options)
                .map_err(|err| VesselError::Storage(format!("zip '{clean}': {err}")))?;
            writer
                .write_all(content.as_bytes())
                .map_err(|err| VesselError::Storage(format!("write zip '{clean}': {err}")))?;
        }

        writer
            .finish()
            .map_err(|err| VesselError::Storage(format!("finalize .plugin.vessel zip: {err}")))?;
    }

    Ok(cursor.into_inner())
}

fn collect_dir_recursive(
    root: &Path,
    current: &Path,
    total_bytes: &mut u64,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> VesselResult<()> {
    let entries = fs::read_dir(current).map_err(|err| {
        VesselError::Storage(format!("read plugin dir {}: {err}", current.display()))
    })?;

    for item in entries {
        let entry = item.map_err(|err| VesselError::Storage(err.to_string()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name == ".git" || name == "node_modules" || name == "target" {
            continue;
        }

        if path.is_dir() {
            collect_dir_recursive(root, &path, total_bytes, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|err| VesselError::Storage(err.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            if rel != "manifest.json" && rel != "icon.png" && !is_text_module_file(&rel) {
                continue;
            }
            let bytes = fs::read(&path).map_err(|err| {
                VesselError::Storage(format!("read plugin file {}: {err}", path.display()))
            })?;
            if bytes.len() as u64 > MAX_PACKAGE_ENTRY_BYTES {
                return Err(VesselError::InvalidInput(format!(
                    "plugin file '{rel}' exceeds max size limit"
                )));
            }
            *total_bytes = total_bytes.saturating_add(bytes.len() as u64);
            if *total_bytes > MAX_PACKAGE_UNCOMPRESSED_BYTES {
                return Err(VesselError::InvalidInput(
                    "local plugin directory exceeds 16 MiB size limit".into(),
                ));
            }
            out.insert(rel, bytes);
        }
    }

    Ok(())
}

/// Loads an unpacked plugin directory from disk (for macOS Dev Mode & local hot-reload).
pub fn load_plugin_dir(dir: &Path) -> VesselResult<LoadedPackage> {
    if !dir.is_dir() {
        return Err(VesselError::NotFound(format!(
            "plugin directory does not exist: {}",
            dir.display()
        )));
    }
    let mut raw_entries = BTreeMap::new();
    let mut total = 0u64;
    collect_dir_recursive(dir, dir, &mut total, &mut raw_entries)?;

    let mut digest_buf = Vec::new();
    for (k, v) in &raw_entries {
        digest_buf.extend_from_slice(k.as_bytes());
        digest_buf.push(0);
        digest_buf.extend_from_slice(v);
    }
    let sha256 = sha256_hex(&digest_buf);

    build_package_from_raw(raw_entries, sha256)
}

/// Parses static `import` / `export ... from` specifiers from a TS/JS file using `oxc_parser`.
///
/// Only relative intra-bundle specifiers (`./...` and `../...`) are returned.
pub fn collect_relative_imports(filename: &str, source: &str) -> VesselResult<Vec<String>> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::ts());
    let ret = Parser::new(&allocator, source, source_type).parse();
    if ret.fatal_error {
        return Err(VesselError::InvalidInput(format!(
            "failed to parse imports in '{filename}'"
        )));
    }

    let mut specs: Vec<String> = ret
        .module_record
        .requested_modules
        .keys()
        .map(|s| s.as_str().to_owned())
        .filter(|spec| spec.starts_with("./") || spec.starts_with("../"))
        .collect();
    specs.sort();
    specs.dedup();
    Ok(specs)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use vessel_domain::{PluginManifest, VesselError};

    use super::{
        collect_relative_imports, load_plugin_dir, load_plugin_package, pack_plugin_package,
        verify_sha256,
    };
    use crate::ts_strip::sha256_hex;

    fn sample_manifest() -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": "anthology",
            "name": "Anthology",
            "version": "1.0.0",
            "entry": "src/index.ts",
            "capabilities": ["vod"],
            "domains": ["anthology.example"]
        }))
        .expect("sample manifest")
    }

    #[test]
    fn packs_and_loads_plugin_vessel_zip_with_sha256_verification() {
        let manifest = sample_manifest();
        let mut files = BTreeMap::new();
        files.insert(
            "src/index.ts".into(),
            "import { parseCard } from './parser'; export default { parseCard };".into(),
        );
        files.insert(
            "src/parser.ts".into(),
            "export const parseCard = (x: string): string => x;".into(),
        );
        let icon = b"\x89PNG\r\n\x1a\nfakepng";

        let archive_bytes =
            pack_plugin_package(&manifest, &files, Some(icon)).expect("pack .plugin.vessel");
        let expected_sha = sha256_hex(&archive_bytes);

        verify_sha256(&archive_bytes, &expected_sha).expect("sha256 matches");
        let loaded =
            load_plugin_package(&archive_bytes, Some(&expected_sha)).expect("load .plugin.vessel");

        assert_eq!(loaded.manifest.id.as_str(), "anthology");
        assert_eq!(loaded.icon_png.as_deref(), Some(icon.as_slice()));
        assert_eq!(
            loaded
                .bundle
                .resolve_entry(&loaded.manifest.entry)
                .expect("entry"),
            "src/index.ts"
        );
        assert!(loaded.bundle.get("src/parser.ts").is_some());

        let bad_sha = "0".repeat(64);
        let err = load_plugin_package(&archive_bytes, Some(&bad_sha))
            .expect_err("reject sha256 mismatch");
        assert!(matches!(err, VesselError::InvalidInput(_)));
    }

    #[test]
    fn loads_dev_mode_directory_and_collects_relative_ts_imports() {
        let temp = std::env::temp_dir().join(format!(
            "vessel-pkg-dir-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(temp.join("src")).expect("mkdir src");
        std::fs::write(
            temp.join("manifest.json"),
            serde_json::to_vec_pretty(&sample_manifest()).expect("json"),
        )
        .expect("write manifest");
        std::fs::write(
            temp.join("src/index.ts"),
            r#"
            import { http } from "vessel";
            import { parse } from "./parser";
            export { helper } from "../shared/helpers";
            "#,
        )
        .expect("write index.ts");

        let loaded = load_plugin_dir(&temp).expect("load local plugin dir");
        assert_eq!(loaded.manifest.id.as_str(), "anthology");

        let src = loaded.bundle.get("src/index.ts").expect("src/index.ts");
        let imports = collect_relative_imports("src/index.ts", src).expect("collect imports");
        assert_eq!(
            imports,
            vec!["../shared/helpers".to_owned(), "./parser".to_owned()]
        );

        let _ = std::fs::remove_dir_all(&temp);
    }
}
