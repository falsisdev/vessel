//! Core domain types, errors, and port traits for Vessel.

pub mod error;
pub mod event;
pub mod ids;
pub mod media;
#[cfg(feature = "http-client")]
pub mod net;
pub mod paths;
pub mod plugin;
pub mod ports;
pub mod stream;
pub mod ui;

pub use error::{VesselError, VesselResult, format_error_chain};
pub use event::{
    DownloadProgressSnapshot, DownloadStatus, EngineEvent, LogLevel, PlayerStateSnapshot,
};
pub use ids::{ExternalIds, MediaRef, PluginId};
pub use media::{
    ContentRating, Episode, EpisodeTag, LinkedMediaSource, MediaDetail, MediaRelation,
    MediaSummary, MediaType, PlaybackProgress, RelationKind, SeasonSummary,
};
#[cfg(feature = "http-client")]
pub use net::http_client_builder;
pub use paths::VesselPaths;
pub use plugin::{
    ContentCategory, PluginCapability, PluginManifest, PluginSettingField, RegistryIndex,
    RepoIndex, RepoPluginEntry, SettingDefault, SettingFieldType, SettingSelectOption,
    SharedExtractorsRef,
};
pub use ports::{
    ContentProvider, DomainGrantSource, EventSink, HomeFeed, HomeSection, HomeSectionLayout,
    LibraryItem, LibraryStatus, PlaybackRequest, PlayerBackend, ResolvedSubtitle, SearchPage,
    SecretStore, StoragePort, SubtitleCandidate, SubtitleProvider, SubtitleQuery,
};
pub use stream::{
    AudioPriority, AudioType, SkipSegment, SkipSegmentType, StreamCandidate, StreamFlag,
    StreamKind, StreamResult, SubtitleFormat, SubtitleSource, SubtitleTrack,
};
pub use ui::{
    AdvancedUiPreferences, CardAspectRatio, CardPreferences, CornerStyle, DetailsPreferences,
    EpisodeListLayout, FavoriteLiveChannel, HeroBannerMode, HomePreferences, HomePreset,
    LiveTvPreferences, LiveTvViewMode, MobileNavLayout, NavLayout, NavModule, PlayClickBehavior,
    PlaybackPreferences, ShellPreferences, ThemeColors, ThemeSpec, UiDensity, UiPreferences,
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
        assert!(!domain_version().is_empty());
        assert_eq!(domain_version(), env!("CARGO_PKG_VERSION"));
    }
}
