//! ThemeSpec tokens and modular UI preferences.

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

/// Shell chrome, sidebar module visibility, and density preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShellPreferences {
    pub layout: NavLayout,
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
        }
    }
}

/// Aspect ratio for media poster cards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAspectRatio {
    #[default]
    Poster2x3,
    Wide16x9,
    Square,
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
}

impl Default for PlaybackPreferences {
    fn default() -> Self {
        Self {
            click_behavior: PlayClickBehavior::default(),
            show_live_control_bar: true,
            auto_next_delay_secs: 10,
            watched_threshold_percent: 90,
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

/// Declarative, portable UI customization configuration backed by SQLite `settings`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UiPreferences {
    pub shell: ShellPreferences,
    pub home: HomePreferences,
    pub cards: CardPreferences,
    pub details: DetailsPreferences,
    pub playback: PlaybackPreferences,
    pub advanced: AdvancedUiPreferences,
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
    }
}
