//! ThemeSpec tokens and modular UI preferences.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{VesselError, VesselResult};

/// Eleven core color tokens shared across desktop CSS variables and Android Compose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColors {
    pub bg_base: String,
    pub bg_surface: String,
    pub bg_elevated: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_secondary: String,
    pub border: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
}

/// Portable `.theme.vessel` color theme specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSpec {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub author: String,
    pub is_dark: bool,
    pub colors: ThemeColors,
}

impl ThemeSpec {
    /// Returns the built-in default `Catppuccin Mocha` theme.
    pub fn catppuccin_mocha() -> Self {
        Self {
            schema_version: 1,
            id: "catppuccin-mocha".into(),
            name: "Catppuccin Mocha".into(),
            author: "Vessel".into(),
            is_dark: true,
            colors: ThemeColors {
                bg_base: "#1e1e2e".into(),
                bg_surface: "#181825".into(),
                bg_elevated: "#313244".into(),
                text_primary: "#cdd6f4".into(),
                text_muted: "#a6adc8".into(),
                accent: "#cba6f7".into(),
                accent_secondary: "#89b4fa".into(),
                border: "#45475a".into(),
                success: "#a6e3a1".into(),
                warning: "#f9e2af".into(),
                danger: "#f38ba8".into(),
            },
        }
    }

    /// Validates basic `.theme.vessel` invariants.
    pub fn validate(&self) -> VesselResult<()> {
        if self.schema_version != 1 {
            return Err(VesselError::InvalidInput(format!(
                "unsupported theme schemaVersion: {}",
                self.schema_version
            )));
        }
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err(VesselError::InvalidInput(
                "theme id and name cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

impl Default for ThemeSpec {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

/// Primary navigation shell layout options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavLayout {
    #[default]
    ExpandedSidebar,
    PinnedSidebar,
    FloatingBottom,
    CollapsedRail,
    TopNavbar,
    Hidden,
}

/// Hideable/reorderable top-level navigation modules (`Settings` stays accessible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavModule {
    Home,
    Search,
    Library,
    LiveTv,
    Downloads,
    Extensions,
}

/// Interface spacing density preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

/// Corner roundness style across cards and controls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    Sharp,
    #[default]
    Rounded,
    Pill,
}

/// Mobile navigation bar style on Android.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileNavLayout {
    #[default]
    Standard,
    FloatingBottom,
}

/// Shell chrome, sidebar module visibility, and density preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShellPreferences {
    pub layout: NavLayout,
    pub mobile_nav_layout: MobileNavLayout,
    pub visible_modules: Vec<NavModule>,
    pub density: UiDensity,
    pub font_scale_percent: u16,
    pub corner_style: CornerStyle,
    pub reduced_motion: bool,
}

impl Default for ShellPreferences {
    fn default() -> Self {
        Self {
            layout: NavLayout::default(),
            mobile_nav_layout: MobileNavLayout::default(),
            visible_modules: vec![
                NavModule::Home,
                NavModule::Search,
                NavModule::Library,
                NavModule::LiveTv,
                NavModule::Downloads,
                NavModule::Extensions,
            ],
            density: UiDensity::default(),
            font_scale_percent: 100,
            corner_style: CornerStyle::default(),
            reduced_motion: false,
        }
    }
}

/// Preset home screen focus profiles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomePreset {
    #[default]
    Default,
    MinimalFocus,
    AnimeFirst,
    IptvHub,
}

/// Hero banner presentation mode on the home screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroBannerMode {
    #[default]
    Full,
    Compact,
    Hidden,
}

/// Home screen rails and section visibility preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HomePreferences {
    pub preset: HomePreset,
    pub hero_banner: HeroBannerMode,
    pub show_continue_watching: bool,
    pub continue_watching_limit: u16,
    pub show_recent_iptv: bool,
    pub hidden_sections: Vec<String>,
    pub selected_plugins: Option<Vec<String>>,
    pub hero_plugins: Option<Vec<String>>,
}

impl Default for HomePreferences {
    fn default() -> Self {
        Self {
            preset: HomePreset::default(),
            hero_banner: HeroBannerMode::default(),
            show_continue_watching: true,
            continue_watching_limit: 12,
            show_recent_iptv: true,
            hidden_sections: Vec::new(),
            selected_plugins: None,
            hero_plugins: None,
        }
    }
}

/// Aspect ratio for media poster cards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardAspectRatio {
    #[default]
    #[serde(rename = "poster_2x3", alias = "poster2x3")]
    Poster2x3,
    #[serde(rename = "portrait_3x4")]
    Portrait3x4,
    #[serde(rename = "portrait_4x5")]
    Portrait4x5,
    #[serde(rename = "square")]
    Square,
    #[serde(rename = "classic_4x3")]
    Classic4x3,
    #[serde(rename = "wide_16x9", alias = "wide16x9")]
    Wide16x9,
    #[serde(rename = "cinema_21x9")]
    Cinema21x9,
}

/// Poster card grid layout and micro-badge toggles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CardPreferences {
    pub aspect_ratio: CardAspectRatio,
    pub columns: Option<u8>,
    pub show_plugin_badge: bool,
    pub show_rating: bool,
    pub show_year: bool,
    pub show_content_type: bool,
    pub show_watched_check: bool,
    pub show_progress_bar: bool,
    pub show_hover_actions: bool,
    pub blur_nsfw_posters: bool,
}

impl Default for CardPreferences {
    fn default() -> Self {
        Self {
            aspect_ratio: CardAspectRatio::default(),
            columns: None,
            show_plugin_badge: true,
            show_rating: true,
            show_year: true,
            show_content_type: true,
            show_watched_check: true,
            show_progress_bar: true,
            show_hover_actions: true,
            blur_nsfw_posters: true,
        }
    }
}

/// Episode list visual presentation on the details page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeListLayout {
    #[default]
    GridCards,
    DetailedList,
    CompactRows,
}

/// Media details screen and spoiler-free options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DetailsPreferences {
    pub show_backdrop: bool,
    pub backdrop_dim_percent: u8,
    pub show_external_id_badges: bool,
    pub show_fix_match: bool,
    pub show_switch_source: bool,
    pub episode_layout: EpisodeListLayout,
    pub hide_episode_thumbnails: bool,
    pub hide_episode_overviews: bool,
}

impl Default for DetailsPreferences {
    fn default() -> Self {
        Self {
            show_backdrop: true,
            backdrop_dim_percent: 65,
            show_external_id_badges: true,
            show_fix_match: true,
            show_switch_source: true,
            episode_layout: EpisodeListLayout::default(),
            hide_episode_thumbnails: false,
            hide_episode_overviews: false,
        }
    }
}

/// Action triggered when clicking Play on an episode or movie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayClickBehavior {
    #[default]
    QuickStartBest,
    AlwaysAskSource,
}

/// Playback interaction and watched-threshold UI preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaybackPreferences {
    pub click_behavior: PlayClickBehavior,
    pub show_live_control_bar: bool,
    pub auto_next_delay_secs: u16,
    pub watched_threshold_percent: u8,
    pub natural_touchpad_scroll: bool,
}

impl Default for PlaybackPreferences {
    fn default() -> Self {
        Self {
            click_behavior: PlayClickBehavior::default(),
            show_live_control_bar: true,
            auto_next_delay_secs: 10,
            watched_threshold_percent: 90,
            natural_touchpad_scroll: false,
        }
    }
}

/// Subtitle outline/background rendering preset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleOutlineStyle {
    #[default]
    Outline,
    Shadow,
    Box,
    None,
}

/// Customizable subtitle typography, color, backdrop, and placement settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubtitlePreferences {
    pub font_size_px: u8,
    pub font_family: String,
    pub text_color: String,
    pub background_opacity: u8,
    pub outline_style: SubtitleOutlineStyle,
    pub bottom_offset_px: u16,
    pub bold: bool,
}

impl Default for SubtitlePreferences {
    fn default() -> Self {
        Self {
            font_size_px: 28,
            font_family: "system-ui".into(),
            text_color: "#ffffff".into(),
            background_opacity: 72,
            outline_style: SubtitleOutlineStyle::Outline,
            bottom_offset_px: 56,
            bold: false,
        }
    }
}

/// Power-user CSS customization preferences.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AdvancedUiPreferences {
    pub custom_css_enabled: bool,
    pub custom_css: String,
}

/// Visual layout preset for the Live TV directory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTvViewMode {
    #[default]
    Grid,
    CompactList,
    ShowcaseTiles,
}

/// Persisted favorite Live TV plugin channel stored on-device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteLiveChannel {
    pub plugin_id: String,
    pub native_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,
}

/// Live TV plugins, view appearance, favorites, and per-plugin category section ordering preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LiveTvPreferences {
    pub plugin_order: Vec<String>,
    pub section_order_by_plugin: BTreeMap<String, Vec<String>>,
    pub favorite_channels: Vec<FavoriteLiveChannel>,
    pub view_mode: LiveTvViewMode,
    pub density: UiDensity,
    pub columns: Option<u8>,
    pub show_logos: bool,
    pub show_categories: bool,
}

impl Default for LiveTvPreferences {
    fn default() -> Self {
        Self {
            plugin_order: Vec::new(),
            section_order_by_plugin: BTreeMap::new(),
            favorite_channels: Vec::new(),
            view_mode: LiveTvViewMode::default(),
            density: UiDensity::default(),
            columns: None,
            show_logos: true,
            show_categories: true,
        }
    }
}

/// Declarative, portable UI customization configuration backed by SQLite `settings`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UiPreferences {
    pub shell: ShellPreferences,
    pub home: HomePreferences,
    pub cards: CardPreferences,
    pub details: DetailsPreferences,
    pub playback: PlaybackPreferences,
    pub subtitles: SubtitlePreferences,
    pub advanced: AdvancedUiPreferences,
    pub live_tv: LiveTvPreferences,
    pub search_history: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{NavLayout, PlayClickBehavior, ThemeSpec, UiPreferences};
    use crate::VesselError;

    #[test]
    fn validates_default_catppuccin_mocha_theme() {
        let theme = ThemeSpec::default();
        theme.validate().expect("default theme should be valid");
        assert_eq!(theme.id, "catppuccin-mocha");
        assert!(theme.is_dark);
        assert_eq!(theme.colors.accent, "#cba6f7");

        let mut bad = theme;
        bad.id = "  ".into();
        assert_eq!(
            bad.validate(),
            Err(VesselError::InvalidInput(
                "theme id and name cannot be empty".into()
            ))
        );
    }

    #[test]
    fn merges_partial_ui_preferences_with_defaults() {
        let partial = r#"{
            "shell": { "layout": "collapsed_rail" },
            "details": { "hideEpisodeThumbnails": true },
            "playback": { "clickBehavior": "always_ask_source" }
        }"#;

        let prefs: UiPreferences =
            serde_json::from_str(partial).expect("parse partial UiPreferences");
        assert_eq!(prefs.shell.layout, NavLayout::CollapsedRail);
        assert_eq!(prefs.shell.font_scale_percent, 100);
        assert!(prefs.details.hide_episode_thumbnails);
        assert!(!prefs.details.hide_episode_overviews);
        assert_eq!(
            prefs.playback.click_behavior,
            PlayClickBehavior::AlwaysAskSource
        );
        assert_eq!(prefs.playback.watched_threshold_percent, 90);
        assert!(prefs.cards.blur_nsfw_posters);
        assert!(prefs.live_tv.show_logos);
        assert!(prefs.live_tv.show_categories);
        assert_eq!(prefs.live_tv.view_mode, super::LiveTvViewMode::Grid);
    }

    #[test]
    fn round_trips_live_tv_preferences_and_favorites() {
        let raw = r#"{
            "liveTv": {
                "pluginOrder": ["anthology_spor"],
                "viewMode": "compact_list",
                "density": "compact",
                "columns": 3,
                "showLogos": false,
                "showCategories": true,
                "favoriteChannels": [
                    {
                        "pluginId": "anthology_spor",
                        "nativeId": "tv:1",
                        "title": "Sports 1",
                        "sectionTitle": "Spor"
                    }
                ]
            }
        }"#;
        let parsed: UiPreferences = serde_json::from_str(raw).expect("parse liveTv prefs");
        assert_eq!(parsed.live_tv.plugin_order, vec!["anthology_spor"]);
        assert_eq!(parsed.live_tv.view_mode, super::LiveTvViewMode::CompactList);
        assert_eq!(parsed.live_tv.density, super::UiDensity::Compact);
        assert_eq!(parsed.live_tv.columns, Some(3));
        assert!(!parsed.live_tv.show_logos);
        assert_eq!(parsed.live_tv.favorite_channels.len(), 1);
        assert_eq!(parsed.live_tv.favorite_channels[0].title, "Sports 1");
    }

    #[test]
    fn round_trips_card_aspect_ratio_canonical_and_legacy_names() {
        use super::CardAspectRatio;

        let wide: UiPreferences =
            serde_json::from_str(r#"{"cards":{"aspectRatio":"wide_16x9"}}"#).expect("wide_16x9");
        assert_eq!(wide.cards.aspect_ratio, CardAspectRatio::Wide16x9);
        let serialized = serde_json::to_string(&wide.cards.aspect_ratio).expect("serialize");
        assert_eq!(serialized, r#""wide_16x9""#);

        let legacy: UiPreferences = serde_json::from_str(r#"{"cards":{"aspectRatio":"wide16x9"}}"#)
            .expect("legacy wide16x9");
        assert_eq!(legacy.cards.aspect_ratio, CardAspectRatio::Wide16x9);

        let poster: UiPreferences =
            serde_json::from_str(r#"{"cards":{"aspectRatio":"poster_2x3"}}"#).expect("poster_2x3");
        assert_eq!(poster.cards.aspect_ratio, CardAspectRatio::Poster2x3);
        assert_eq!(
            serde_json::to_string(&poster.cards.aspect_ratio).expect("serialize poster"),
            r#""poster_2x3""#
        );

        for (raw, expected) in [
            ("portrait_3x4", CardAspectRatio::Portrait3x4),
            ("portrait_4x5", CardAspectRatio::Portrait4x5),
            ("classic_4x3", CardAspectRatio::Classic4x3),
            ("cinema_21x9", CardAspectRatio::Cinema21x9),
            ("square", CardAspectRatio::Square),
        ] {
            let parsed: UiPreferences =
                serde_json::from_str(&format!(r#"{{"cards":{{"aspectRatio":"{raw}"}}}}"#))
                    .expect("parse extended ratio");
            assert_eq!(parsed.cards.aspect_ratio, expected);
        }
    }
}
