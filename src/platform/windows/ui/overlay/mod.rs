//! Overlay rendering module.

pub mod renderer;

pub use renderer::{
    create_arial_bold_font_face, release_render_cache, update_overlay, D2D_FACTORY, DWRITE_FACTORY,
    FONT_FACE,
};
