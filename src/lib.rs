//! Core library entrypoint exporting domain types and rendering utilities.

pub mod core;
pub mod image;

pub use core::{
    AuditEvent, CardDeck, CardMeta, CardRecord, CardType, ColumnRange, Deck, DeckHeader,
    EncodingKind, Ibm029Encoder, PunchCard, PunchEncoding, RenderStyle, Template, TemplateRegistry,
    ValidChar,
};
pub use image::{
    CardImageStyle, GLYPH_HEIGHT, GLYPH_WIDTH, ImageRenderOptions, PageLayout, render_card_image,
};

use anyhow::Result;

/// Splits the entire input text into 80-column punch cards and encodes them.
pub fn encode_text_to_deck<E: PunchEncoding>(
    encoder: &E,
    text: &str,
    with_seq_numbers: bool,
) -> Result<CardDeck> {
    CardDeck::from_text(encoder, text, with_seq_numbers)
}
