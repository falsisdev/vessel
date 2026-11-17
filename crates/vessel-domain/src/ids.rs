//! Strongly typed identifiers and external ID mappings.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{VesselError, VesselResult};

/// Strongly typed plugin identifier (Newtype over `String`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginId(String);

impl PluginId {
    /// Validates and constructs a non-empty plugin identifier.
    pub fn new(raw: impl Into<String>) -> VesselResult<Self> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(VesselError::InvalidInput(
                "plugin id cannot be empty".into(),
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrows the identifier as a string slice without allocating.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PluginId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Primary self-contained media identifier: `(plugin_id, native_id)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRef {
    pub plugin_id: PluginId,
    pub native_id: String,
}

impl MediaRef {
    /// Creates a new `MediaRef` after ensuring `native_id` is non-empty.
    pub fn new(plugin_id: PluginId, native_id: impl Into<String>) -> VesselResult<Self> {
        let trimmed = native_id.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(VesselError::InvalidInput(
                "native id cannot be empty".into(),
            ));
        }
        Ok(Self {
            plugin_id,
            native_id: trimmed,
        })
    }
}

/// Optional cross-provider external ID hints for metadata enrichment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExternalIds {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmdb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imdb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tvdb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anilist: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitsu: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mal: Option<u64>,
}

impl ExternalIds {
    /// Returns `true` when no external provider ID is set.
    pub fn is_empty(&self) -> bool {
        self.tmdb.is_none()
            && self.imdb.is_none()
            && self.tvdb.is_none()
            && self.anilist.is_none()
            && self.kitsu.is_none()
            && self.mal.is_none()
    }

    /// Exports populated IDs in deterministic key order.
    pub fn to_map(&self) -> BTreeMap<&'static str, String> {
        let mut map = BTreeMap::new();
        if let Some(id) = self.tmdb {
            map.insert("tmdb", id.to_string());
        }
        if let Some(id) = &self.imdb {
            map.insert("imdb", id.clone());
        }
        if let Some(id) = self.tvdb {
            map.insert("tvdb", id.to_string());
        }
        if let Some(id) = self.anilist {
            map.insert("anilist", id.to_string());
        }
        if let Some(id) = self.kitsu {
            map.insert("kitsu", id.to_string());
        }
        if let Some(id) = self.mal {
            map.insert("mal", id.to_string());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::{ExternalIds, MediaRef, PluginId};
    use crate::VesselError;

    #[test]
    fn rejects_empty_plugin_or_native_ids() {
        assert_eq!(
            PluginId::new("   "),
            Err(VesselError::InvalidInput(
                "plugin id cannot be empty".into()
            ))
        );

        let plugin_id = PluginId::new("  anthology ").expect("valid plugin id");
        assert_eq!(plugin_id.as_str(), "anthology");
        assert_eq!(
            MediaRef::new(plugin_id, ""),
            Err(VesselError::InvalidInput(
                "native id cannot be empty".into()
            ))
        );
    }

    #[test]
    fn serializes_media_ref_and_external_ids_cleanly() {
        let plugin_id = PluginId::new("anthology").expect("valid plugin id");
        let media_ref = MediaRef::new(plugin_id, "/anime/frieren").expect("valid media ref");
        let json = serde_json::to_string(&media_ref).expect("serialize MediaRef");
        assert_eq!(
            json,
            r#"{"pluginId":"anthology","nativeId":"/anime/frieren"}"#
        );

        let ids: ExternalIds =
            serde_json::from_str(r#"{"anilist":154587,"imdb":"tt22248376"}"#).expect("parse ids");
        assert!(!ids.is_empty());
        assert_eq!(
            ids.to_map().get("anilist").map(String::as_str),
            Some("154587")
        );
        assert_eq!(
            ids.to_map().get("imdb").map(String::as_str),
            Some("tt22248376")
        );
    }
}
