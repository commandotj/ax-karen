//! User settings in `~/.karen/config.toml`, read and written only by the CLI.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const POWER_PRESETS: [&str; 4] = ["miser", "balanced", "aggressive", "drain"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Scan worker preset: miser, balanced, aggressive or drain.
    pub power: String,
    /// Interface zoom factor, 0.6 to 2.0.
    pub ui_zoom: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            power: "balanced".to_owned(),
            ui_zoom: 1.0,
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(io::Error),
    Parse(toml::de::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Parse(error) => write!(formatter, "{error}"),
        }
    }
}

impl Settings {
    /// `~/.karen/config.toml`; `None` when no home directory is known.
    pub fn default_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(PathBuf::from(home).join(".karen").join("config.toml"))
    }

    /// A missing file means defaults. A file that exists but cannot be parsed
    /// is an error, so a hand edit is never silently overwritten.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str::<Self>(&text)
                .map(Self::sanitized)
                .map_err(SettingsError::Parse),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(Self::default())
            }
            Err(error) => Err(SettingsError::Io(error)),
        }
    }

    /// Writes through a temporary file so a crash never leaves half a config.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(&self.clone().sanitized())
            .map_err(|error| io::Error::other(error.to_string()))?;
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, text)?;
        fs::rename(&temporary, path)
    }

    fn sanitized(mut self) -> Self {
        if !POWER_PRESETS.contains(&self.power.as_str()) {
            self.power = Self::default().power;
        }
        self.ui_zoom = if self.ui_zoom.is_finite() {
            self.ui_zoom.clamp(0.6, 2.0)
        } else {
            1.0
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_and_values_round_trip() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(".karen").join("config.toml");
        assert_eq!(
            Settings::load(&path).expect("defaults"),
            Settings::default()
        );

        let settings = Settings {
            power: "miser".to_owned(),
            ui_zoom: 1.2,
        };
        settings.save(&path).expect("save");
        assert_eq!(Settings::load(&path).expect("reload"), settings);
        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[test]
    fn unknown_values_are_clamped_and_broken_files_are_errors() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "power = \"turbo\"\nui_zoom = 9.0\n").expect("write");
        let loaded = Settings::load(&path).expect("sanitized");
        assert_eq!((loaded.power.as_str(), loaded.ui_zoom), ("balanced", 2.0));

        fs::write(&path, "power = [broken").expect("write");
        assert!(matches!(
            Settings::load(&path),
            Err(SettingsError::Parse(_))
        ));
    }
}
