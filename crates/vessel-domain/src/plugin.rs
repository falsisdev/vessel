//! Plugin manifest, repository index, shortcode registry, and settings schema models.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::PluginId;
use crate::{VesselError, VesselResult};

/// Declared runtime role of a `.plugin.vessel` package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Vod,
    Search,
    LiveTv,
    Subtitles,
    Extractor,
}

/// Catalog/discovery category advertised by a plugin for UI filtering and metadata routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentCategory {
    Movie,
    Series,
    Anime,
    Live,
}

/// Supported input widget types for plugin `settingsSchema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingFieldType {
    Text,
    Secret,
    Toggle,
    Select,
}

/// Choice entry for a `Select` field in plugin `settingsSchema`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingSelectOption {
    pub label: String,
    pub value: String,
}

/// Typed default value for a plugin setting field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingDefault {
    Bool(bool),
    Text(String),
}

/// Declarative per-plugin setting rendered dynamically by UI shells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: SettingFieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<SettingDefault>,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub options: Vec<SettingSelectOption>,
}

/// Identity card (`manifest.json`) of a plugin package or local dev folder.
///
/// Intentionally omits any `badge` field: the `Official` badge is derived strictly
/// by the core engine from trusted sources or signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: PluginId,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_core_version: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    #[serde(default)]
    pub content_types: Vec<ContentCategory>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub allow_any_https: bool,
    #[serde(default)]
    pub is_adult: bool,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub settings_schema: Vec<PluginSettingField>,
}

impl PluginManifest {
    /// Validates basic structural invariants of a parsed plugin manifest.
    pub fn validate(&self) -> VesselResult<()> {
        if self.schema_version != 1 {
            return Err(VesselError::InvalidInput(format!(
                "unsupported manifest schemaVersion: {}",
                self.schema_version
            )));
        }
        if self.id.as_str().trim().is_empty() {
            return Err(VesselError::InvalidInput(
                "manifest id cannot be empty".into(),
            ));
        }
        if self.entry.trim().is_empty() {
            return Err(VesselError::InvalidInput(
                "manifest entry cannot be empty".into(),
            ));
        }
        if self.capabilities.is_empty() {
            return Err(VesselError::InvalidInput(
                "manifest must declare at least one capability".into(),
            ));
        }
        Ok(())
    }
}

/// Shared extractor bundle reference declared at repository level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedExtractorsRef {
    pub id: PluginId,
    pub version: String,
    pub download_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Single plugin listing inside a `repo.json` index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoPluginEntry {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    #[serde(default)]
    pub content_types: Vec<ContentCategory>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub is_adult: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_core_version: Option<String>,
}

/// Multi-plugin repository index (`repo.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoIndex {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default)]
    pub plugin_lists: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_extractors: Option<SharedExtractorsRef>,
    #[serde(default)]
    pub plugins: Vec<RepoPluginEntry>,
}

/// Static shortcode dictionary (`registry.json`) mapping short names to source inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIndex {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub codes: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::{
        ContentCategory, PluginCapability, PluginManifest, RegistryIndex, RepoIndex,
        SettingDefault, SettingFieldType,
    };
    use crate::VesselError;

    #[test]
    fn parses_and_validates_manifest_with_settings_schema() {
        let raw = r#"{
            "schemaVersion": 1,
            "id": "anthology",
            "name": "Anthology",
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["vod", "search", "subtitles"],
            "contentTypes": ["anime", "series"],
            "languages": ["tr", "en"],
            "domains": ["anthology.example"],
            "badge": "Official",
            "settingsSchema": [
                { "key": "mirrorUrl", "label": "Mirror URL", "type": "text", "default": "https://anthology.example" },
                { "key": "apiKey", "label": "API Key", "type": "secret" },
                { "key": "autoVerify", "label": "Auto Verify", "type": "toggle", "default": true }
            ]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(raw).expect("parse PluginManifest");
        manifest.validate().expect("valid manifest");
        assert_eq!(manifest.id.as_str(), "anthology");
        assert_eq!(
            manifest.capabilities,
            vec![
                PluginCapability::Vod,
                PluginCapability::Search,
                PluginCapability::Subtitles
            ]
        );
        assert_eq!(
            manifest.content_types,
            vec![ContentCategory::Anime, ContentCategory::Series]
        );
        assert_eq!(
            manifest.settings_schema[1].field_type,
            SettingFieldType::Secret
        );
        assert_eq!(
            manifest.settings_schema[2].default,
            Some(SettingDefault::Bool(true))
        );
    }

    #[test]
    fn rejects_invalid_manifest_and_parses_repo_and_registry() {
        let mut manifest: PluginManifest = serde_json::from_str(
            r#"{
                "schemaVersion": 2,
                "id": "anthology",
                "name": "Anthology",
                "version": "1.0.0",
                "entry": "index.ts",
                "capabilities": ["vod"]
            }"#,
        )
        .expect("parse v2 manifest");
        assert_eq!(
            manifest.validate(),
            Err(VesselError::InvalidInput(
                "unsupported manifest schemaVersion: 2".into()
            ))
        );
        manifest.schema_version = 1;
        manifest.capabilities.clear();
        assert_eq!(
            manifest.validate(),
            Err(VesselError::InvalidInput(
                "manifest must declare at least one capability".into()
            ))
        );

        let repo: RepoIndex = serde_json::from_str(
            r#"{
                "schemaVersion": 1,
                "id": "dev.falsis.main",
                "name": "Vessel Anthology Repo",
                "plugins": [{
                    "id": "anthology",
                    "name": "Anthology",
                    "version": "1.0.0",
                    "capabilities": ["vod", "search"],
                    "sourcePath": "plugins/anthology"
                }]
            }"#,
        )
        .expect("parse RepoIndex");
        assert_eq!(repo.plugins[0].id.as_str(), "anthology");

        let registry: RegistryIndex = serde_json::from_str(
            r#"{
                "schemaVersion": 1,
                "codes": { "anthology": "gh:falsisdev/anthology" }
            }"#,
        )
        .expect("parse RegistryIndex");
        assert_eq!(
            registry.codes.get("anthology").map(String::as_str),
            Some("gh:falsisdev/anthology")
        );
    }
}
