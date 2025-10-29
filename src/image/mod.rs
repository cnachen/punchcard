//! Rendering helpers for producing PNG output of punch cards.

mod paint;

pub use paint::{
    CardImageStyle, GLYPH_HEIGHT, GLYPH_WIDTH, ImageRenderOptions, PageLayout, render_card_image,
};
