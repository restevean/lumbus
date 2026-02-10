//! Drawing functions for the overlay using Direct2D.

/// Drawing parameters for the overlay.
#[derive(Clone)]
pub struct DrawParams {
    /// Circle radius in pixels.
    pub radius: f32,
    /// Border width in pixels.
    pub border_width: f32,
    /// Stroke color (r, g, b, a) normalized 0.0-1.0.
    pub stroke_color: (f32, f32, f32, f32),
    /// Fill alpha (0.0-1.0).
    pub fill_alpha: f32,
}

/// What to draw: circle or click letter.
#[derive(Clone, Copy, PartialEq)]
pub enum ClickLetter {
    None,
    Left,
    Right,
}

/// Draw a circle at the specified position.
pub fn draw_circle(_params: &DrawParams, _x: f32, _y: f32) {
    // TODO: Implement with Direct2D
}

/// Draw a letter (L or R) at the specified position.
pub fn draw_letter(
    _letter: ClickLetter,
    _params: &DrawParams,
    _x: f32,
    _y: f32,
    _is_spanish: bool,
) {
    // TODO: Implement with Direct2D
}
