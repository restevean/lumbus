//! JSON configuration storage for Windows.
//!
//! Stores settings in %APPDATA%/Lumbus/config.json

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration stored in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Circle radius in pixels.
    #[serde(default = "default_radius")]
    pub radius: f64,

    /// Border width in pixels.
    #[serde(default = "default_border")]
    pub border_width: f64,

    /// Stroke color (r, g, b, a) normalized 0.0-1.0.
    #[serde(default = "default_color")]
    pub stroke_color: (f64, f64, f64, f64),

    /// Fill transparency percentage (0-100).
    #[serde(default = "default_transparency")]
    pub fill_transparency: i32,

    /// Language: "en" or "es".
    #[serde(default = "default_lang")]
    pub lang: String,

    /// Whether overlay is enabled.
    #[serde(default = "default_enabled")]
    pub overlay_enabled: bool,
}

fn default_radius() -> f64 {
    50.0
}
fn default_border() -> f64 {
    3.0
}
fn default_color() -> (f64, f64, f64, f64) {
    (1.0, 0.84, 0.0, 1.0) // Gold
}
fn default_transparency() -> i32 {
    50
}
fn default_lang() -> String {
    "en".to_string()
}
fn default_enabled() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            radius: default_radius(),
            border_width: default_border(),
            stroke_color: default_color(),
            fill_transparency: default_transparency(),
            lang: default_lang(),
            overlay_enabled: default_enabled(),
        }
    }
}

impl Config {
    /// Get the config file path.
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "restevean", "Lumbus")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    /// Load configuration from JSON file.
    /// Returns default config if file doesn't exist or is invalid.
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save configuration to JSON file.
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_path() {
            // Create directory if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.radius, 50.0);
        assert_eq!(config.border_width, 3.0);
        assert_eq!(config.lang, "en");
        assert!(config.overlay_enabled);
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let loaded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config.radius, loaded.radius);
        assert_eq!(config.lang, loaded.lang);
    }
}
