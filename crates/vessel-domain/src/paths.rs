//! Canonical `<VesselRoot>` directory layout resolver (pure path computation, zero I/O).

use std::path::{Path, PathBuf};

use crate::{PluginId, VesselError, VesselResult};

/// Resolved filesystem layout anchored at `<VesselRoot>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselPaths {
    root: PathBuf,
    data_dir: PathBuf,
}

impl VesselPaths {
    /// Creates a standard desktop layout where internal data lives under `<root>/data`.
    pub fn new(root: impl Into<PathBuf>) -> VesselResult<Self> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(VesselError::InvalidInput(
                "vessel root path cannot be empty".into(),
            ));
        }
        let data_dir = root.join("data");
        Ok(Self { root, data_dir })
    }

    /// Creates a split layout (such as Android) with user-visible root and app-private `data_dir`.
    pub fn with_data_dir(
        root: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
    ) -> VesselResult<Self> {
        let root = root.into();
        let data_dir = data_dir.into();
        if root.as_os_str().is_empty() || data_dir.as_os_str().is_empty() {
            return Err(VesselError::InvalidInput(
                "vessel root and data_dir paths cannot be empty".into(),
            ));
        }
        Ok(Self { root, data_dir })
    }

    /// Resolves `<home>/Vessel` from a caller-provided home directory.
    pub fn from_home_dir(home: &Path) -> VesselResult<Self> {
        if home.as_os_str().is_empty() {
            return Err(VesselError::InvalidInput(
                "home directory path cannot be empty".into(),
            ));
        }
        Self::new(home.join("Vessel"))
    }

    /// Resolves `~/Vessel` (`HOME`) or `%USERPROFILE%\Vessel` from process environment variables.
    pub fn from_home_env() -> VesselResult<Self> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .ok_or_else(|| {
                VesselError::NotFound("home directory environment variable not set".into())
            })?;
        Self::from_home_dir(&home)
    }

    /// Returns the `<VesselRoot>` base directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns `<VesselRoot>/downloads` for offline video packages and subtitles.
    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// Returns `<VesselRoot>/backups` for `.backup.vessel` exports.
    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// Returns `<VesselRoot>/locales` for user/community language JSON packs.
    pub fn locales_dir(&self) -> PathBuf {
        self.root.join("locales")
    }

    /// Returns `<VesselRoot>/themes` for custom `.theme.vessel` theme files.
    pub fn themes_dir(&self) -> PathBuf {
        self.root.join("themes")
    }

    /// Returns `<VesselRoot>/plugins` for installed `.plugin.vessel` packages.
    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }

    /// Returns the installation directory for a specific plugin ID.
    pub fn plugin_dir(&self, plugin_id: &PluginId) -> PathBuf {
        self.plugins_dir().join(plugin_id.as_str())
    }

    /// Returns the internal application data directory (`<VesselRoot>/data` on desktop).
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Returns the SQLite database file path (`<data_dir>/vessel.db`).
    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join("vessel.db")
    }

    /// Returns the six standard directories expected under a provisioned `<VesselRoot>`.
    pub fn required_dirs(&self) -> [PathBuf; 6] {
        [
            self.downloads_dir(),
            self.backups_dir(),
            self.locales_dir(),
            self.themes_dir(),
            self.plugins_dir(),
            self.data_dir.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::VesselPaths;
    use crate::{PluginId, VesselError};

    #[test]
    fn resolves_standard_desktop_vessel_root_layout() {
        let paths = VesselPaths::from_home_dir(Path::new("/Users/falsis")).expect("valid home");
        assert_eq!(paths.root(), Path::new("/Users/falsis/Vessel"));
        assert_eq!(
            paths.downloads_dir(),
            Path::new("/Users/falsis/Vessel/downloads")
        );
        assert_eq!(
            paths.backups_dir(),
            Path::new("/Users/falsis/Vessel/backups")
        );
        assert_eq!(
            paths.locales_dir(),
            Path::new("/Users/falsis/Vessel/locales")
        );
        assert_eq!(paths.themes_dir(), Path::new("/Users/falsis/Vessel/themes"));
        assert_eq!(
            paths.plugins_dir(),
            Path::new("/Users/falsis/Vessel/plugins")
        );
        assert_eq!(paths.data_dir(), Path::new("/Users/falsis/Vessel/data"));
        assert_eq!(
            paths.database_file(),
            Path::new("/Users/falsis/Vessel/data/vessel.db")
        );

        let anthology = PluginId::new("anthology").expect("valid plugin id");
        assert_eq!(
            paths.plugin_dir(&anthology),
            Path::new("/Users/falsis/Vessel/plugins/anthology")
        );
        assert_eq!(paths.required_dirs().len(), 6);
    }

    #[test]
    fn supports_split_data_dir_and_rejects_empty_paths() {
        let android_paths = VesselPaths::with_data_dir(
            "/storage/emulated/0/Vessel",
            "/data/user/0/dev.falsis.vessel/files",
        )
        .expect("valid split layout");

        assert_eq!(
            android_paths.downloads_dir(),
            Path::new("/storage/emulated/0/Vessel/downloads")
        );
        assert_eq!(
            android_paths.database_file(),
            Path::new("/data/user/0/dev.falsis.vessel/files/vessel.db")
        );

        assert_eq!(
            VesselPaths::new(""),
            Err(VesselError::InvalidInput(
                "vessel root path cannot be empty".into()
            ))
        );
    }
}
