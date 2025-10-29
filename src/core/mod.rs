//! Core domain primitives for punch card decks, encoding, and templates.

pub mod deck;
pub mod encoding;
pub mod punchcards;
pub mod templates;

pub use deck::{
    AuditEvent, CardMeta, CardRecord, CardType, ColumnRange, Deck, DeckHeader, EncodingKind,
};
pub use encoding::{Ibm029Encoder, PunchEncoding, ValidChar};
pub use punchcards::{CardDeck, PunchCard, RenderStyle};
pub use templates::{Template, TemplateRegistry};
