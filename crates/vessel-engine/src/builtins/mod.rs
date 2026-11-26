//! Built-in first-party plugins embedded directly into the Vessel engine binary.

use std::collections::BTreeMap;

use vessel_domain::VesselResult;
use vessel_plugin_rt::{LoadedPackage, load_builtin_package};

pub const BUILTIN_SOURCE_TYPE: &str = "builtin";
pub const SUBTITLES_CONTAINER_ID: &str = "subtitles-container";

const SUBTITLES_CONTAINER_MANIFEST: &str = include_str!("subtitles_container/manifest.json");
const SUBTITLES_CONTAINER_INDEX: &str = include_str!("subtitles_container/index.ts");

/// Returns all built-in plugins that Vessel provisions automatically on startup.
pub fn builtin_packages() -> VesselResult<Vec<LoadedPackage>> {
    let mut files = BTreeMap::new();
    files.insert("index.ts".to_owned(), SUBTITLES_CONTAINER_INDEX.to_owned());
    let subtitles_pkg = load_builtin_package(SUBTITLES_CONTAINER_MANIFEST, files)?;
    Ok(vec![subtitles_pkg])
}
