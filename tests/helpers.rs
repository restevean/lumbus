use mouse_highlighter::{clamp, color_to_hex, parse_hex_color, tr_key};

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn clamp_keeps_inner_value() {
    assert_eq!(clamp(10.0, 0.0, 20.0), 10.0);
}

#[test]
fn clamp_limits_low_and_high() {
    assert_eq!(clamp(-1.0, 0.0, 1.0), 0.0);
    assert_eq!(clamp(2.0, 0.0, 1.0), 1.0);
}

#[test]
fn color_to_hex_without_alpha_when_opaque() {
    let hex = color_to_hex(1.0, 0.0, 0.5, 1.0);
    assert_eq!(hex, "#FF0080");
}

#[test]
fn color_to_hex_with_alpha_when_not_opaque() {
    let hex = color_to_hex(0.2, 0.4, 0.6, 0.5);
    assert_eq!(hex, "#33669980");
}

#[test]
fn color_to_hex_clamps_input() {
    let hex = color_to_hex(-0.1, 1.2, 0.501, 1.0);
    assert_eq!(hex, "#00FF80");
}

#[test]
fn parse_hex_rgb() {
    let (r, g, b, a) = parse_hex_color("#FF0080").expect("valid rgb hex");
    assert!(approx_eq(r, 1.0));
    assert!(approx_eq(g, 0.0));
    assert!(approx_eq(b, 128.0 / 255.0));
    assert!(approx_eq(a, 1.0));
}

#[test]
fn parse_hex_rgba() {
    let (r, g, b, a) = parse_hex_color("#33669980").expect("valid rgba hex");
    assert!(approx_eq(r, 51.0 / 255.0));
    assert!(approx_eq(g, 102.0 / 255.0));
    assert!(approx_eq(b, 153.0 / 255.0));
    assert!(approx_eq(a, 128.0 / 255.0));
}

#[test]
fn parse_hex_trims_and_ignores_whitespace() {
    let (r, g, b, a) = parse_hex_color("  ff00FF80  ").expect("valid with whitespace and mixed case");
    assert!(approx_eq(r, 1.0));
    assert!(approx_eq(g, 0.0));
    assert!(approx_eq(b, 1.0));
    assert!(approx_eq(a, 128.0 / 255.0));
}

#[test]
fn parse_hex_invalid_lengths_return_none() {
    assert!(parse_hex_color("#FFF").is_none());
    assert!(parse_hex_color("#FF00").is_none());
    assert!(parse_hex_color("#FF00FF0000").is_none());
}

#[test]
fn parse_hex_invalid_chars_return_none() {
    assert!(parse_hex_color("#GG0000").is_none());
    assert!(parse_hex_color("ZZZZZZZZ").is_none());
}

#[test]
fn roundtrip_rgb_exact_when_opaque() {
    let rgb = (51.0 / 255.0, 102.0 / 255.0, 153.0 / 255.0);
    let hex = color_to_hex(rgb.0, rgb.1, rgb.2, 1.0);
    assert_eq!(hex, "#336699");
    let (r2, g2, b2, a2) = parse_hex_color(&hex).unwrap();
    assert!(approx_eq(r2, rgb.0));
    assert!(approx_eq(g2, rgb.1));
    assert!(approx_eq(b2, rgb.2));
    assert!(approx_eq(a2, 1.0));
}

#[test]
fn roundtrip_rgba_including_alpha() {
    let rgba = (
        17.0 / 255.0,
        34.0 / 255.0,
        51.0 / 255.0,
        204.0 / 255.0,
    );
    let hex = color_to_hex(rgba.0, rgba.1, rgba.2, rgba.3);
    assert_eq!(hex, "#112233CC");
    let (r2, g2, b2, a2) = parse_hex_color(&hex).unwrap();
    assert!(approx_eq(r2, rgba.0));
    assert!(approx_eq(g2, rgba.1));
    assert!(approx_eq(b2, rgba.2));
    assert!(approx_eq(a2, rgba.3));
}

#[test]
fn tr_key_localisation_en_es() {
    assert_eq!(tr_key("Settings", false).as_ref(), "Settings");
    assert_eq!(tr_key("Settings", true).as_ref(), "Configuración");

    assert_eq!(tr_key("Close", false).as_ref(), "Close");
    assert_eq!(tr_key("Close", true).as_ref(), "Cerrar");

    // Fallback for unknown key
    assert_eq!(tr_key("UnknownKey", false).as_ref(), "UnknownKey");
    assert_eq!(tr_key("UnknownKey", true).as_ref(), "UnknownKey");
}
