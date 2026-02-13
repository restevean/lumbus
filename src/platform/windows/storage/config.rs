//! JSON configuration file for Windows.
//!
//! Stores settings in %APPDATA%/Lumbus/config.json
//!
//! Uses an in-memory cache to avoid disk I/O on every slider change.
//! Call `flush_config()` to persist changes to disk.

use crate::model::constants::*;
use crate::model::OverlayState;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

/// Serializable config structure for JSON persistence.
#[derive(Serialize, Deserialize, Debug, Clone)]
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

// In-memory config cache. Loaded once, written on flush.
thread_local! {
    static CONFIG_CACHE: RefCell<Option<Config>> = const { RefCell::new(None) };
    static CONFIG_DIRTY: RefCell<bool> = const { RefCell::new(false) };
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
fn load_config_from_disk() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Save config to JSON file.
fn save_config_to_disk(config: &Config) {
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

/// Get the cached config, loading from disk if needed.
fn get_config() -> Config {
    CONFIG_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(load_config_from_disk());
        }
        cache.clone().unwrap()
    })
}

/// Update the cached config and mark it dirty.
fn set_config(config: Config) {
    CONFIG_CACHE.with(|cache| {
        *cache.borrow_mut() = Some(config);
    });
    CONFIG_DIRTY.with(|dirty| {
        *dirty.borrow_mut() = true;
    });
}

/// Flush the config cache to disk if dirty.
///
/// Call this when settings window closes or app exits.
pub fn flush_config() {
    let is_dirty = CONFIG_DIRTY.with(|dirty| *dirty.borrow());
    if !is_dirty {
        return;
    }

    CONFIG_CACHE.with(|cache| {
        if let Some(ref config) = *cache.borrow() {
            save_config_to_disk(config);
        }
    });

    CONFIG_DIRTY.with(|dirty| {
        *dirty.borrow_mut() = false;
    });
}

/// Load state from config file.
pub fn load_state() -> OverlayState {
    let config = get_config();
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
///
/// Note: This updates the cache immediately but only writes to disk
/// when `flush_config()` is called.
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
    set_config(config);
}

/// Read a double from config (from cache).
pub fn prefs_get_double(key: &str, default: f64) -> f64 {
    let config = get_config();
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

/// Write a double to config (to cache, flush later).
pub fn prefs_set_double(key: &str, val: f64) {
    let mut config = get_config();
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
    set_config(config);
}

/// Read an integer from config (from cache).
pub fn prefs_get_int(key: &str, default: i32) -> i32 {
    let config = get_config();
    match key {
        PREF_LANG => config.lang,
        _ => default,
    }
}

/// Write an integer to config (to cache, flush later).
pub fn prefs_set_int(key: &str, val: i32) {
    let mut config = get_config();
    match key {
        PREF_LANG => config.lang = val,
        _ => return,
    }
    set_config(config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert!((config.radius - DEFAULT_DIAMETER / 2.0).abs() < f64::EPSILON);
        assert_eq!(config.lang, LANG_EN);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = Config {
            radius: 42.0,
            border_width: 3.0,
            stroke_r: 0.5,
            stroke_g: 0.6,
            stroke_b: 0.7,
            stroke_a: 1.0,
            fill_transparency_pct: 50.0,
            lang: LANG_ES,
        };
        let json = serde_json::to_string(&config).unwrap();
        let loaded: Config = serde_json::from_str(&json).unwrap();
        assert!((loaded.radius - 42.0).abs() < f64::EPSILON);
        assert_eq!(loaded.lang, LANG_ES);
    }
}
