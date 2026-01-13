#![allow(unexpected_cfgs)] // Silence cfg warnings from objc macros

//! Pure helpers used by the app. Keep this file free of macOS FFI so tests
//! can run as normal integration tests.

pub mod events;
pub mod model;

use std::borrow::Cow;

// Re-export model types for convenience
pub use model::OverlayState;

// Re-export event types for convenience
pub use events::{AppEvent, EventBus, EventPublisher};

/// Clamp a value to [lo, hi]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

/// Convert RGBA floats [0..1] to #RRGGBB or #RRGGBBAA (if alpha < 1).
pub fn color_to_hex(r: f64, g: f64, b: f64, a: f64) -> String {
    let ri = (clamp(r, 0.0, 1.0) * 255.0).round() as u8;
    let gi = (clamp(g, 0.0, 1.0) * 255.0).round() as u8;
    let bi = (clamp(b, 0.0, 1.0) * 255.0).round() as u8;
    let ai = (clamp(a, 0.0, 1.0) * 255.0).round() as u8;
    if ai == 255 {
        format!("#{:02X}{:02X}{:02X}", ri, gi, bi)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", ri, gi, bi, ai)
    }
}

/// Parse `#RRGGBB` or `#RRGGBBAA` into normalised floats [0..1].
pub fn parse_hex_color(s: &str) -> Option<(f64, f64, f64, f64)> {
    let t = s.trim();
    let t = t.strip_prefix('#').unwrap_or(t);
    let hex = t.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let (r, g, b, a) = match hex.len() {
        6 => {
            let rv = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let gv = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let bv = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (rv, gv, bv, 255u8)
        }
        8 => {
            let rv = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let gv = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let bv = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let av = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (rv, gv, bv, av)
        }
        _ => return None,
    };
    Some((
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    ))
}

/// Very small localisation helper used in Settings.
pub fn tr_key(key: &str, es: bool) -> Cow<'static, str> {
    match (key, es) {
        ("Settings", true) => Cow::Borrowed("Configuración"),
        ("Settings", false) => Cow::Borrowed("Settings"),

        ("Language", true) => Cow::Borrowed("Idioma"),
        ("Language", false) => Cow::Borrowed("Language"),

        ("English", true) => Cow::Borrowed("Inglés"),
        ("English", false) => Cow::Borrowed("English"),

        ("Spanish", true) => Cow::Borrowed("Español"),
        ("Spanish", false) => Cow::Borrowed("Spanish"),

        ("Radius (px)", true) => Cow::Borrowed("Radio (px)"),
        ("Radius (px)", false) => Cow::Borrowed("Radius (px)"),

        ("Border (px)", true) => Cow::Borrowed("Grosor (px)"),
        ("Border (px)", false) => Cow::Borrowed("Border (px)"),

        ("Color", true) => Cow::Borrowed("Color"),
        ("Color", false) => Cow::Borrowed("Color"),

        ("Hex", true) => Cow::Borrowed("Hex"),
        ("Hex", false) => Cow::Borrowed("Hex"),

        ("Fill Transparency (%)", true) => Cow::Borrowed("Transparencia (%)"),
        ("Fill Transparency (%)", false) => Cow::Borrowed("Fill Transparency (%)"),

        ("Close", true) => Cow::Borrowed("Cerrar"),
        ("Close", false) => Cow::Borrowed("Close"),

        ("Quit", true) => Cow::Borrowed("Salir"),
        ("Quit", false) => Cow::Borrowed("Quit"),
        ("Cancel", true) => Cow::Borrowed("Cancelar"),
        ("Cancel", false) => Cow::Borrowed("Cancel"),

        // Help overlay
        ("Help", true) => Cow::Borrowed("Ayuda"),
        ("Help", false) => Cow::Borrowed("Help"),

        ("Keyboard Shortcuts", true) => Cow::Borrowed("Atajos de teclado"),
        ("Keyboard Shortcuts", false) => Cow::Borrowed("Keyboard Shortcuts"),

        ("Toggle overlay", true) => Cow::Borrowed("Mostrar/ocultar resaltado"),
        ("Toggle overlay", false) => Cow::Borrowed("Toggle overlay"),

        ("Open settings", true) => Cow::Borrowed("Abrir configuración"),
        ("Open settings", false) => Cow::Borrowed("Open settings"),

        ("Show help", true) => Cow::Borrowed("Mostrar ayuda"),
        ("Show help", false) => Cow::Borrowed("Show help"),

        ("Quit app", true) => Cow::Borrowed("Salir de la app"),
        ("Quit app", false) => Cow::Borrowed("Quit app"),

        ("Press any key to close", true) => Cow::Borrowed("Pulsa cualquier tecla para cerrar"),
        ("Press any key to close", false) => Cow::Borrowed("Press any key to close"),

        _ => Cow::Owned(key.to_string()),
    }
}
