//! Core domain types, errors, and port traits for Vessel.

pub mod error;
pub mod event;
pub mod ids;
pub mod media;
pub mod plugin;
pub mod ports;
pub mod stream;
pub mod ui;

pub use error::{VesselError, VesselResult};
pub use ids::{ExternalIds, MediaRef, PluginId};
pub use media::{
    ContentRating, Episode, EpisodeTag, MediaDetail, MediaRelation, MediaSummary, MediaType,
    PlaybackProgress, RelationKind, SeasonSummary,
};

/// Returns the crate version embedded at compile time from `Cargo.toml`.
pub fn domain_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use crate::domain_version;

    #[test]
    fn domain_version_matches_workspace() {
        assert_eq!(domain_version(), "0.1.0");
    }
}
