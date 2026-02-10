//! JSON configuration file for Windows.

use crate::model::constants::*;
use crate::model::OverlayState;

/// Get config file path: %APPDATA%/Lumbus/config.json
fn config_path() -> std::path::PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(appdata)
        .join("Lumbus")
        .join("config.json")
}

/// Load state from config file.
pub fn load_state() -> OverlayState {
    // TODO: Implement JSON loading
    let mut state = OverlayState {
        radius: DEFAULT_DIAMETER / 2.0,
        border_width: DEFAULT_BORDER_WIDTH,
        stroke_r: DEFAULT_COLOR.0,
        stroke_g: DEFAULT_COLOR.1,
        stroke_b: DEFAULT_COLOR.2,
        stroke_a: DEFAULT_COLOR.3,
        fill_transparency_pct: DEFAULT_FILL_TRANSPARENCY_PCT,
        lang: LANG_EN,
        overlay_enabled: true,
        display_mode: DISPLAY_MODE_CIRCLE,
    };
    state.validate();
    state
}

/// Save state to config file.
pub fn save_state(_state: &OverlayState) {
    // TODO: Implement JSON saving
}

/// Read a double from config.
pub unsafe fn prefs_get_double(_key: &str, default: f64) -> f64 {
    // TODO: Implement
    default
}

/// Write a double to config.
pub unsafe fn prefs_set_double(_key: &str, _val: f64) {
    // TODO: Implement
}

/// Read an integer from config.
pub unsafe fn prefs_get_int(_key: &str, default: i32) -> i32 {
    // TODO: Implement
    default
}

/// Write an integer to config.
pub unsafe fn prefs_set_int(_key: &str, _val: i32) {
    // TODO: Implement
}
