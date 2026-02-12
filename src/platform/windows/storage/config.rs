//! JSON configuration file for Windows.
//!
//! Stores settings in %APPDATA%/Lumbus/config.json

use crate::model::constants::*;
use crate::model::OverlayState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Serializable config structure for JSON persistence.
#[derive(Serialize, Deserialize, Debug)]
struct Config {
    radius: f64,
    border_width: f64,
    stroke_r: f64,
    stroke_g: f64,
    stroke_b: f64,
    stroke_a: f64,
    fill_transparency_pct: f64,
    lang: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            radius: DEFAULT_DIAMETER / 2.0,
            border_width: DEFAULT_BORDER_WIDTH,
            stroke_r: DEFAULT_COLOR.0,
            stroke_g: DEFAULT_COLOR.1,
            stroke_b: DEFAULT_COLOR.2,
            stroke_a: DEFAULT_COLOR.3,
            fill_transparency_pct: DEFAULT_FILL_TRANSPARENCY_PCT,
            lang: LANG_EN,
        }
    }
}

/// Get config file path: %APPDATA%/Lumbus/config.json
fn config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("Lumbus").join("config.json")
}

/// Ensure the config directory exists.
fn ensure_config_dir() -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Load config from JSON file, returning defaults if not found or invalid.
fn load_config() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Save config to JSON file.
fn save_config(config: &Config) {
    if ensure_config_dir().is_err() {
        eprintln!("Failed to create config directory");
        return;
    }

    let path = config_path();
    match serde_json::to_string_pretty(config) {
        Ok(json) => {
            if let Err(e) = fs::write(&path, json) {
                eprintln!("Failed to write config: {}", e);
            }
        }
        Err(e) => eprintln!("Failed to serialize config: {}", e),
    }
}

/// Load state from config file.
pub fn load_state() -> OverlayState {
    let config = load_config();
    let mut state = OverlayState {
        radius: config.radius,
        border_width: config.border_width,
        stroke_r: config.stroke_r,
        stroke_g: config.stroke_g,
        stroke_b: config.stroke_b,
        stroke_a: config.stroke_a,
        fill_transparency_pct: config.fill_transparency_pct,
        lang: config.lang,
        overlay_enabled: true,
        display_mode: DISPLAY_MODE_CIRCLE,
    };
    state.validate();
    state
}

/// Save state to config file.
pub fn save_state(state: &OverlayState) {
    let config = Config {
        radius: state.radius,
        border_width: state.border_width,
        stroke_r: state.stroke_r,
        stroke_g: state.stroke_g,
        stroke_b: state.stroke_b,
        stroke_a: state.stroke_a,
        fill_transparency_pct: state.fill_transparency_pct,
        lang: state.lang,
    };
    save_config(&config);
}

/// Read a double from config.
pub fn prefs_get_double(key: &str, default: f64) -> f64 {
    let config = load_config();
    match key {
        PREF_RADIUS => config.radius,
        PREF_BORDER => config.border_width,
        PREF_STROKE_R => config.stroke_r,
        PREF_STROKE_G => config.stroke_g,
        PREF_STROKE_B => config.stroke_b,
        PREF_STROKE_A => config.stroke_a,
        PREF_FILL_TRANSPARENCY => config.fill_transparency_pct,
        _ => default,
    }
}

/// Write a double to config.
pub fn prefs_set_double(key: &str, val: f64) {
    let mut config = load_config();
    match key {
        PREF_RADIUS => config.radius = val,
        PREF_BORDER => config.border_width = val,
        PREF_STROKE_R => config.stroke_r = val,
        PREF_STROKE_G => config.stroke_g = val,
        PREF_STROKE_B => config.stroke_b = val,
        PREF_STROKE_A => config.stroke_a = val,
        PREF_FILL_TRANSPARENCY => config.fill_transparency_pct = val,
        _ => return,
    }
    save_config(&config);
}

/// Read an integer from config.
pub fn prefs_get_int(key: &str, default: i32) -> i32 {
    let config = load_config();
    match key {
        PREF_LANG => config.lang,
        _ => default,
    }
}

/// Write an integer to config.
pub fn prefs_set_int(key: &str, val: i32) {
    let mut config = load_config();
    match key {
        PREF_LANG => config.lang = val,
        _ => return,
    }
    save_config(&config);
}
