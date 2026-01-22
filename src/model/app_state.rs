//! Application state (pure Rust, no FFI).
//!
//! This module defines the overlay state structure that can be
//! serialized to/from NSUserDefaults.

use super::constants::*;
use crate::clamp;

/// Complete overlay state, serializable to/from NSUserDefaults.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayState {
    /// Circle radius in pixels.
    pub radius: f64,
    /// Border width in pixels.
    pub border_width: f64,
    /// Stroke color - red component [0.0, 1.0].
    pub stroke_r: f64,
    /// Stroke color - green component [0.0, 1.0].
    pub stroke_g: f64,
    /// Stroke color - blue component [0.0, 1.0].
    pub stroke_b: f64,
    /// Stroke color - alpha component [0.0, 1.0].
    pub stroke_a: f64,
    /// Fill transparency [0.0, 100.0] (100 = fully transparent).
    pub fill_transparency_pct: f64,
    /// Language: 0 = EN, 1 = ES.
    pub lang: i32,
    /// Is overlay globally enabled?
    pub overlay_enabled: bool,
    /// Display mode: 0 = circle, 1 = L, 2 = R.
    pub display_mode: i32,
}

impl Default for OverlayState {
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
            overlay_enabled: true,
            display_mode: DISPLAY_MODE_CIRCLE,
        }
    }
}

impl OverlayState {
    /// Validates and clamps all values to valid ranges.
    pub fn validate(&mut self) {
        self.radius = clamp(self.radius, MIN_RADIUS, MAX_RADIUS);
        self.border_width = clamp(self.border_width, MIN_BORDER, MAX_BORDER);
        self.fill_transparency_pct = clamp(
            self.fill_transparency_pct,
            MIN_TRANSPARENCY,
            MAX_TRANSPARENCY,
        );
        self.stroke_r = clamp(self.stroke_r, 0.0, 1.0);
        self.stroke_g = clamp(self.stroke_g, 0.0, 1.0);
        self.stroke_b = clamp(self.stroke_b, 0.0, 1.0);
        self.stroke_a = clamp(self.stroke_a, 0.0, 1.0);
    }

    /// Returns the stroke color as a tuple (r, g, b, a).
    pub fn stroke_color(&self) -> (f64, f64, f64, f64) {
        (self.stroke_r, self.stroke_g, self.stroke_b, self.stroke_a)
    }

    /// Calculates fill alpha based on transparency.
    /// 0% transparency = alpha 1.0, 100% transparency = alpha 0.0
    pub fn fill_alpha(&self) -> f64 {
        1.0 - (self.fill_transparency_pct / 100.0)
    }

    /// Returns true if current language is Spanish.
    pub fn is_spanish(&self) -> bool {
        self.lang == LANG_ES
    }
}
