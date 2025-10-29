mod deck;
mod encoding;
mod punchcards;
mod templates;

pub use deck::{
    AuditEvent, CardMeta, CardRecord, CardType, ColumnRange, Deck, DeckHeader, EncodingKind,
};
pub use encoding::{Ibm029Encoder, PunchEncoding, ValidChar};
pub use punchcards::{CardDeck, PunchCard, RenderStyle};
pub use templates::{Template, TemplateRegistry};

use anyhow::Result;

/// Splits the entire input text into 80-column punch cards and encodes them.
pub fn encode_text_to_deck<E: PunchEncoding>(
    encoder: &E,
    text: &str,
    with_seq_numbers: bool,
) -> Result<CardDeck> {
    CardDeck::from_text(encoder, text, with_seq_numbers)
}
