//! Tests for the model layer (OverlayState).
//!
//! Note: We intentionally use `Default::default()` then field reassignment
//! to test individual field validation. This is clearer than struct update syntax.
#![allow(clippy::field_reassign_with_default)]

use lumbus::model::app_state::OverlayState;
use lumbus::model::constants::*;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// === Default Values Tests ===

#[test]
fn overlay_state_default_radius() {
    let state = OverlayState::default();
    assert!(approx_eq(state.radius, DEFAULT_DIAMETER / 2.0));
}

#[test]
fn overlay_state_default_border_width() {
    let state = OverlayState::default();
    assert!(approx_eq(state.border_width, DEFAULT_BORDER_WIDTH));
}

#[test]
fn overlay_state_default_stroke_color() {
    let state = OverlayState::default();
    assert!(approx_eq(state.stroke_r, DEFAULT_COLOR.0));
    assert!(approx_eq(state.stroke_g, DEFAULT_COLOR.1));
    assert!(approx_eq(state.stroke_b, DEFAULT_COLOR.2));
    assert!(approx_eq(state.stroke_a, DEFAULT_COLOR.3));
}

#[test]
fn overlay_state_default_fill_transparency() {
    let state = OverlayState::default();
    assert!(approx_eq(
        state.fill_transparency_pct,
        DEFAULT_FILL_TRANSPARENCY_PCT
    ));
}

#[test]
fn overlay_state_default_lang_is_english() {
    let state = OverlayState::default();
    assert_eq!(state.lang, LANG_EN);
}

#[test]
fn overlay_state_default_overlay_enabled() {
    let state = OverlayState::default();
    assert!(state.overlay_enabled);
}

#[test]
fn overlay_state_default_display_mode_is_circle() {
    let state = OverlayState::default();
    assert_eq!(state.display_mode, DISPLAY_MODE_CIRCLE);
}

// === Validation Tests ===

#[test]
fn validate_clamps_radius_below_minimum() {
    let mut state = OverlayState::default();
    state.radius = 1.0; // below MIN_RADIUS
    state.validate();
    assert!(approx_eq(state.radius, MIN_RADIUS));
}

#[test]
fn validate_clamps_radius_above_maximum() {
    let mut state = OverlayState::default();
    state.radius = 500.0; // above MAX_RADIUS
    state.validate();
    assert!(approx_eq(state.radius, MAX_RADIUS));
}

#[test]
fn validate_keeps_radius_in_range() {
    let mut state = OverlayState::default();
    state.radius = 50.0; // valid value
    state.validate();
    assert!(approx_eq(state.radius, 50.0));
}

#[test]
fn validate_clamps_border_below_minimum() {
    let mut state = OverlayState::default();
    state.border_width = 0.5; // below MIN_BORDER
    state.validate();
    assert!(approx_eq(state.border_width, MIN_BORDER));
}

#[test]
fn validate_clamps_border_above_maximum() {
    let mut state = OverlayState::default();
    state.border_width = 50.0; // above MAX_BORDER
    state.validate();
    assert!(approx_eq(state.border_width, MAX_BORDER));
}

#[test]
fn validate_clamps_transparency_below_minimum() {
    let mut state = OverlayState::default();
    state.fill_transparency_pct = -50.0;
    state.validate();
    assert!(approx_eq(state.fill_transparency_pct, MIN_TRANSPARENCY));
}

#[test]
fn validate_clamps_transparency_above_maximum() {
    let mut state = OverlayState::default();
    state.fill_transparency_pct = 150.0;
    state.validate();
    assert!(approx_eq(state.fill_transparency_pct, MAX_TRANSPARENCY));
}

#[test]
fn validate_clamps_stroke_r_below_zero() {
    let mut state = OverlayState::default();
    state.stroke_r = -0.5;
    state.validate();
    assert!(approx_eq(state.stroke_r, 0.0));
}

#[test]
fn validate_clamps_stroke_r_above_one() {
    let mut state = OverlayState::default();
    state.stroke_r = 1.5;
    state.validate();
    assert!(approx_eq(state.stroke_r, 1.0));
}

#[test]
fn validate_clamps_stroke_g() {
    let mut state = OverlayState::default();
    state.stroke_g = -0.5;
    state.validate();
    assert!(approx_eq(state.stroke_g, 0.0));
}

#[test]
fn validate_clamps_stroke_b() {
    let mut state = OverlayState::default();
    state.stroke_b = 2.0;
    state.validate();
    assert!(approx_eq(state.stroke_b, 1.0));
}

#[test]
fn validate_clamps_stroke_a() {
    let mut state = OverlayState::default();
    state.stroke_a = -1.0;
    state.validate();
    assert!(approx_eq(state.stroke_a, 0.0));
}

// === Helper Method Tests ===

#[test]
fn stroke_color_returns_tuple() {
    let state = OverlayState::default();
    let (r, g, b, a) = state.stroke_color();
    assert!(approx_eq(r, state.stroke_r));
    assert!(approx_eq(g, state.stroke_g));
    assert!(approx_eq(b, state.stroke_b));
    assert!(approx_eq(a, state.stroke_a));
}

#[test]
fn fill_alpha_zero_when_fully_transparent() {
    let mut state = OverlayState::default();
    state.fill_transparency_pct = 100.0;
    assert!(approx_eq(state.fill_alpha(), 0.0));
}

#[test]
fn fill_alpha_one_when_zero_transparency() {
    let mut state = OverlayState::default();
    state.fill_transparency_pct = 0.0;
    assert!(approx_eq(state.fill_alpha(), 1.0));
}

#[test]
fn fill_alpha_half_when_fifty_percent() {
    let mut state = OverlayState::default();
    state.fill_transparency_pct = 50.0;
    assert!(approx_eq(state.fill_alpha(), 0.5));
}

#[test]
fn is_spanish_true_for_spanish() {
    let mut state = OverlayState::default();
    state.lang = LANG_ES;
    assert!(state.is_spanish());
}

#[test]
fn is_spanish_false_for_english() {
    let state = OverlayState::default();
    assert!(!state.is_spanish());
}

// === Clone and PartialEq Tests ===

#[test]
fn overlay_state_is_cloneable() {
    let state = OverlayState::default();
    let cloned = state.clone();
    assert_eq!(state, cloned);
}

#[test]
fn overlay_state_equality() {
    let state1 = OverlayState::default();
    let mut state2 = OverlayState::default();
    assert_eq!(state1, state2);

    state2.radius = 100.0;
    assert_ne!(state1, state2);
}
