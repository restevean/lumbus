//! Configuration constants and default values.
//!
//! This module contains all application constants including visual defaults,
//! NSUserDefaults keys, and validation limits.

// === Visual Defaults ===

/// Default circle radius in pixels.
pub const DEFAULT_RADIUS: f64 = 35.0;

/// Default circle diameter in pixels (kept for backwards compatibility).
pub const DEFAULT_DIAMETER: f64 = DEFAULT_RADIUS * 2.0;

/// Default border width in pixels.
pub const DEFAULT_BORDER_WIDTH: f64 = 2.0;

/// Default stroke color (R, G, B, A) - orange #E37B00, fully opaque.
/// E3=227, 7B=123, 00=0 → normalized to 0.0-1.0
pub const DEFAULT_COLOR: (f64, f64, f64, f64) = (
    227.0 / 255.0, // R: 0.890
    123.0 / 255.0, // G: 0.482
    0.0 / 255.0,   // B: 0.0
    1.0,           // A: fully opaque
);

/// Default fill transparency percentage (90 = 90% transparent, 10% visible fill).
pub const DEFAULT_FILL_TRANSPARENCY_PCT: f64 = 90.0;

// === NSUserDefaults Keys ===

/// Key for circle radius preference.
pub const PREF_RADIUS: &str = "radius";

/// Key for border width preference.
pub const PREF_BORDER: &str = "borderWidth";

/// Key for stroke red component preference.
pub const PREF_STROKE_R: &str = "strokeR";

/// Key for stroke green component preference.
pub const PREF_STROKE_G: &str = "strokeG";

/// Key for stroke blue component preference.
pub const PREF_STROKE_B: &str = "strokeB";

/// Key for stroke alpha component preference.
pub const PREF_STROKE_A: &str = "strokeA";

/// Key for fill transparency percentage preference.
pub const PREF_FILL_TRANSPARENCY: &str = "fillTransparencyPct";

/// Key for language preference (0 = EN, 1 = ES).
pub const PREF_LANG: &str = "lang";

// === Validation Limits ===

/// Minimum radius value in pixels.
pub const MIN_RADIUS: f64 = 5.0;

/// Maximum radius value in pixels.
pub const MAX_RADIUS: f64 = 200.0;

/// Radius slider step in pixels.
pub const RADIUS_STEP: f64 = 5.0;

/// Minimum border width in pixels.
pub const MIN_BORDER: f64 = 1.0;

/// Maximum border width in pixels.
pub const MAX_BORDER: f64 = 20.0;

/// Border slider step in pixels.
pub const BORDER_STEP: f64 = 1.0;

/// Minimum fill transparency percentage.
pub const MIN_TRANSPARENCY: f64 = 0.0;

/// Maximum fill transparency percentage.
pub const MAX_TRANSPARENCY: f64 = 100.0;

/// Transparency slider step percentage.
pub const TRANSPARENCY_STEP: f64 = 5.0;

// === Display Modes ===

/// Display mode: show circle around cursor.
pub const DISPLAY_MODE_CIRCLE: i32 = 0;

/// Display mode: show "L" for left click.
pub const DISPLAY_MODE_LEFT: i32 = 1;

/// Display mode: show "R" for right click.
pub const DISPLAY_MODE_RIGHT: i32 = 2;

// === Languages ===

/// Language code for English.
pub const LANG_EN: i32 = 0;

/// Language code for Spanish.
pub const LANG_ES: i32 = 1;
