//! Windows runtime state management.
//!
//! Contains the application state struct and thread-local storage.

use std::cell::RefCell;

use windows::Win32::Foundation::HWND;

use crate::model::constants::*;

/// Windows-specific runtime state.
///
/// Contains both persistent settings (loaded from config) and transient
/// window state (hwnd, dimensions). The settings fields mirror
/// `model::OverlayState` but use `f32` for colors as required by Direct2D.
///
/// Note: This is intentionally separate from `model::OverlayState` to avoid
/// type conversion overhead during rendering. Settings are synced via
/// `config::load_state()` which converts f64 -> f32.
#[allow(dead_code)]
pub struct WindowsRuntimeState {
    // Window-specific fields (not persisted)
    pub hwnd: HWND,
    pub width: i32,
    pub height: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    /// DPI scale factor (1.0 = 100%, 1.25 = 125%, etc.)
    pub dpi_scale: f32,

    // Settings fields (persisted via config.json)
    // Note: colors are f32 for Direct2D compatibility
    pub radius: f64,
    pub border_width: f64,
    pub stroke_r: f32,
    pub stroke_g: f32,
    pub stroke_b: f32,
    pub stroke_a: f32,
    pub fill_transparency_pct: f64,
    pub lang: i32,

    // Runtime state (not persisted)
    pub visible: bool,
    pub display_mode: i32,
}

impl Default for WindowsRuntimeState {
    fn default() -> Self {
        Self {
            hwnd: HWND::default(),
            width: 0,
            height: 0,
            offset_x: 0,
            offset_y: 0,
            dpi_scale: 1.0,
            radius: DEFAULT_DIAMETER / 2.0,
            border_width: DEFAULT_BORDER_WIDTH,
            stroke_r: DEFAULT_COLOR.0 as f32,
            stroke_g: DEFAULT_COLOR.1 as f32,
            stroke_b: DEFAULT_COLOR.2 as f32,
            stroke_a: DEFAULT_COLOR.3 as f32,
            fill_transparency_pct: DEFAULT_FILL_TRANSPARENCY_PCT,
            lang: LANG_EN,
            visible: true,
            display_mode: DISPLAY_MODE_CIRCLE,
        }
    }
}

thread_local! {
    /// Global application state for the Windows overlay.
    pub static STATE: RefCell<WindowsRuntimeState> = RefCell::new(WindowsRuntimeState::default());
}

/// Reload settings from JSON config file into the thread-local state.
pub fn reload_settings_from_config() {
    use crate::platform::windows::storage::config;

    let loaded = config::load_state();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.radius = loaded.radius;
        state.border_width = loaded.border_width;
        state.stroke_r = loaded.stroke_r as f32;
        state.stroke_g = loaded.stroke_g as f32;
        state.stroke_b = loaded.stroke_b as f32;
        state.stroke_a = loaded.stroke_a as f32;
        state.fill_transparency_pct = loaded.fill_transparency_pct;
        state.lang = loaded.lang;
    });
}
