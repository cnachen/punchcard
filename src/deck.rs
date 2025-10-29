use crate::encoding::{EncodeError, PunchEncoding};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

const DECK_VERSION: u8 = 1;
const MAX_COLS: usize = 80;

/// Inclusive column range that can be marked as protected.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnRange {
    pub start: usize,
    pub end: usize,
}

impl ColumnRange {
    pub fn new(start: usize, end: usize) -> Result<Self> {
        if start == 0 || end == 0 || start > end || end > MAX_COLS {
            return Err(anyhow!(
                "column range must satisfy 1 <= start <= end <= {}",
                MAX_COLS
            ));
        }
        Ok(Self { start, end })
    }

    pub fn contains(&self, col: usize) -> bool {
        col >= self.start && col <= self.end
    }
}

/// Label for the intent or provenance of a card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Code,
    Data,
    Jcl,
    Comment,
    Separator,
    Patch,
}

impl Default for CardType {
    fn default() -> Self {
        CardType::Code
    }
}

/// Extra metadata such as color or inline notes.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CardMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Encoding choices made while capturing the card.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EncodingKind {
    Hollerith,
    Ascii,
    Ebcdic,
}

impl Default for EncodingKind {
    fn default() -> Self {
        EncodingKind::Hollerith
    }
}

/// Single card stored in a deck file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardRecord {
    pub text: Option<String>,
    #[serde(default)]
    pub punches: Option<String>,
    #[serde(default)]
    pub encoding: EncodingKind,
    #[serde(default)]
    pub seq: Option<usize>,
    #[serde(default)]
    pub card_type: CardType,
    #[serde(default)]
    pub protected_cols: Vec<ColumnRange>,
    #[serde(default)]
    pub meta: CardMeta,
}

impl CardRecord {
    /// Construct a card from user-provided text, padding to 80 columns and retaining metadata.
    pub fn from_text<S: Into<String>>(
        text: S,
        encoding: EncodingKind,
        card_type: CardType,
    ) -> Result<Self> {
        let text = text.into();
        let normalized = normalize_card_text(&text)?;
        Ok(Self {
            text: Some(normalized),
            punches: None,
            encoding,
            seq: None,
            card_type,
            protected_cols: Vec::new(),
            meta: CardMeta::default(),
        })
    }

    /// Update the optional sequence number attached to the card.
    pub fn ensure_seq(&mut self, seq: Option<usize>) {
        self.seq = seq;
    }

    /// Materialize a [`PunchCard`](crate::punchcards::PunchCard) representation using the supplied encoder.
    pub fn to_punch_card<E: PunchEncoding + ?Sized>(
        &self,
        encoder: &E,
    ) -> Result<crate::punchcards::PunchCard, EncodeError> {
        let text = self.text.as_deref().unwrap_or_else(|| "");
        crate::punchcards::PunchCard::from_str(encoder, text)
    }
}

/// Per-deck metadata stored as a header record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeckHeader {
    pub version: u8,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub protected_cols: Vec<ColumnRange>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub history: Vec<AuditEvent>,
}

impl DeckHeader {
    /// Create a new header with optional language/template metadata.
    pub fn new(
        language: Option<String>,
        template: Option<String>,
        protected_cols: Vec<ColumnRange>,
    ) -> Self {
        Self {
            version: DECK_VERSION,
            created_at: Utc::now(),
            language,
            template,
            protected_cols,
            readonly: false,
            history: Vec::new(),
        }
    }
}

/// Describes how the deck has changed over time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
}

impl AuditEvent {
    /// Create an audit entry using the OS user (if available).
    pub fn new<S: Into<String>>(action: S) -> Self {
        let actor = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        Self {
            timestamp: Utc::now(),
            actor,
            action: action.into(),
        }
    }
}

/// In-memory representation of a deck file.
#[derive(Debug, Clone)]
pub struct Deck {
    pub header: DeckHeader,
    pub cards: Vec<CardRecord>,
    pub path: Option<PathBuf>,
}

impl Deck {
    /// Create an empty deck using the provided header metadata.
    pub fn new(header: DeckHeader) -> Self {
        Self {
            header,
            cards: Vec::new(),
            path: None,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .open(path)
            .with_context(|| format!("failed to open deck file {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow!("deck file {} is empty", path.display()))??;
        let deck_line: DeckLine = serde_json::from_str(&header_line)
            .with_context(|| format!("failed to parse deck header in {}", path.display()))?;
        let header = match deck_line {
            DeckLine::Header(header) => header,
            DeckLine::Card(_) => return Err(anyhow!("expected deck header as first line")),
        };

        let mut cards = Vec::new();
        for (idx, raw) in lines.enumerate() {
            let raw = raw?;
            if raw.trim().is_empty() {
                continue;
            }
            let line: DeckLine = serde_json::from_str(&raw).with_context(|| {
                format!(
                    "failed to parse card record at line {} in {}",
                    idx + 2,
                    path.display()
                )
            })?;
            match line {
                DeckLine::Header(_) => {
                    return Err(anyhow!(
                        "multiple deck headers found in {} at line {}",
                        path.display(),
                        idx + 2
                    ));
                }
                DeckLine::Card(card) => cards.push(card),
            }
        }

        Ok(Self {
            header,
            cards,
            path: Some(path.to_path_buf()),
        })
    }

    pub fn save(&mut self, path: &Path) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("failed to write deck file {}", path.display()))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &DeckLine::Header(self.header.clone()))
            .context("failed to serialize deck header")?;
        writer.write_all(b"\n")?;
        for card in &self.cards {
            serde_json::to_writer(&mut writer, &DeckLine::Card(card.clone()))
                .context("failed to serialize deck card")?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    /// Append a card to the deck, enforcing protected-column constraints.
    pub fn append_card(&mut self, card: CardRecord) -> Result<()> {
        self.enforce_protection(None, &card)?;
        self.cards.push(card);
        Ok(())
    }

    pub fn insert_card(&mut self, index: usize, card: CardRecord) -> Result<()> {
        if index > self.cards.len() {
            return Err(anyhow!(
                "card index {} out of range 0..={}",
                index,
                self.cards.len()
            ));
        }
        self.enforce_protection(None, &card)?;
        self.cards.insert(index, card);
        Ok(())
    }

    /// Replace a card at the specified zero-based index.
    pub fn replace_card(&mut self, index: usize, card: CardRecord) -> Result<()> {
        if index >= self.cards.len() {
            return Err(anyhow!(
                "card index {} out of range 0..{}",
                index,
                self.cards.len().saturating_sub(1)
            ));
        }
        let original = &self.cards[index];
        self.enforce_protection(Some(original), &card)?;
        self.cards[index] = card;
        Ok(())
    }

    /// Create a new deck from a contiguous range of cards.
    pub fn slice(&self, range: std::ops::Range<usize>) -> Result<Self> {
        if range.end > self.cards.len() {
            return Err(anyhow!(
                "slice end {} exceeds deck length {}",
                range.end,
                self.cards.len()
            ));
        }
        let mut new = Self::new(self.header.clone());
        new.cards = self.cards[range].to_vec();
        Ok(new)
    }

    /// Populate sequence numbers and update the 73–80 columns accordingly.
    pub fn number_sequence(&mut self, start: usize, step: usize) {
        let mut value = start;
        for card in &mut self.cards {
            card.seq = Some(value);
            if let Some(text) = card.text.as_mut() {
                let mut chars: Vec<char> = text.chars().collect();
                while chars.len() < MAX_COLS {
                    chars.push(' ');
                }
                let seq_str = format!("{:>8}", value);
                let start_idx = MAX_COLS.saturating_sub(seq_str.len());
                for (offset, ch) in seq_str.chars().enumerate() {
                    let idx = start_idx + offset;
                    if idx < chars.len() {
                        chars[idx] = ch;
                    }
                }
                *text = chars.into_iter().collect();
            }
            value += step;
        }
    }

    pub fn sort_by_sequence(&mut self) {
        self.cards.sort_by(|a, b| match (a.seq, b.seq) {
            (Some(sa), Some(sb)) => sa.cmp(&sb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    /// Compute a SHA-256 hash representing deck contents.
    pub fn hash(&self) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut buffer = Vec::new();
        serde_json::to_writer(&mut buffer, &DeckLine::Header(self.header.clone()))
            .context("failed to hash deck header")?;
        hasher.update(&buffer);
        buffer.clear();
        for card in &self.cards {
            serde_json::to_writer(&mut buffer, &DeckLine::Card(card.clone()))?;
            hasher.update(&buffer);
            buffer.clear();
        }
        let digest = hasher.finalize();
        Ok(format!("{digest:02x}"))
    }

    /// Append an audit log entry.
    pub fn log_action<S: Into<String>>(&mut self, action: S) {
        self.header.history.push(AuditEvent::new(action));
    }

    /// Render cards as 80-column strings, padding blanks for empty cards.
    pub fn as_text(&self) -> Vec<String> {
        self.cards
            .iter()
            .map(|card| card.text.clone().unwrap_or_else(|| " ".repeat(MAX_COLS)))
            .collect()
    }

    pub fn to_punch_deck(
        &self,
        encoder: &dyn PunchEncoding,
    ) -> Result<crate::punchcards::CardDeck, EncodeError> {
        let mut cards = Vec::with_capacity(self.cards.len());
        for card in &self.cards {
            let rendered = card.to_punch_card(encoder)?;
            cards.push(rendered);
        }
        Ok(crate::punchcards::CardDeck { cards })
    }

    /// Merge cards and history from another deck after validating compatibility.
    pub fn merge_from(&mut self, other: &Deck) -> Result<()> {
        if self.header.protected_cols != other.header.protected_cols {
            return Err(anyhow!(
                "protected columns mismatch between decks ({} vs {})",
                format_ranges(&self.header.protected_cols),
                format_ranges(&other.header.protected_cols)
            ));
        }
        if self.header.template != other.header.template {
            return Err(anyhow!("templates differ between decks"));
        }
        if self.header.language != other.header.language {
            return Err(anyhow!("languages differ between decks"));
        }
        self.cards.extend(other.cards.clone());
        self.header.history.extend_from_slice(&other.header.history);
        Ok(())
    }

    /// Construct a new deck from an arbitrary set of card indices.
    pub fn slice_indices(&self, indices: &[usize]) -> Result<Self> {
        let mut new = Self::new(self.header.clone());
        for &idx in indices {
            if idx >= self.cards.len() {
                return Err(anyhow!(
                    "card index {} out of range 0..{}",
                    idx,
                    self.cards.len().saturating_sub(1)
                ));
            }
            new.cards.push(self.cards[idx].clone());
        }
        Ok(new)
    }

    /// Guard protected columns from modification to preserve sequence numbers or constants.
    fn enforce_protection(
        &self,
        original: Option<&CardRecord>,
        updated: &CardRecord,
    ) -> Result<()> {
        if self.header.protected_cols.is_empty() {
            return Ok(());
        }
        let new_text = updated
            .text
            .as_deref()
            .ok_or_else(|| anyhow!("card without text cannot be checked for protection"))?;
        let old_text = original.and_then(|c| c.text.as_deref());
        for range in &self.header.protected_cols {
            for col in range.start..=range.end {
                let idx = col - 1;
                let new_char = new_text
                    .chars()
                    .nth(idx)
                    .ok_or_else(|| anyhow!("card text shorter than {} columns", col))?;
                if let Some(old) = old_text {
                    let old_char = old
                        .chars()
                        .nth(idx)
                        .ok_or_else(|| anyhow!("card text shorter than {} columns", col))?;
                    if new_char != old_char {
                        return Err(anyhow!(
                            "column {} is protected; attempted to change '{}' -> '{}'",
                            col,
                            old_char,
                            new_char
                        ));
                    }
                } else if new_char != ' ' {
                    return Err(anyhow!(
                        "column {} is protected; new cards must leave it blank",
                        col
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DeckLine {
    Header(DeckHeader),
    Card(CardRecord),
}

fn normalize_card_text(text: &str) -> Result<String> {
    let mut buffer: VecDeque<char> = text.chars().collect();
    if buffer.len() > MAX_COLS {
        return Err(anyhow!(
            "card text must not exceed {} columns (got {})",
            MAX_COLS,
            buffer.len()
        ));
    }
    while buffer.len() < MAX_COLS {
        buffer.push_back(' ');
    }
    Ok(buffer.into_iter().collect())
}

impl fmt::Display for EncodingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingKind::Hollerith => write!(f, "hollerith"),
            EncodingKind::Ascii => write!(f, "ascii"),
            EncodingKind::Ebcdic => write!(f, "ebcdic"),
        }
    }
}

fn format_ranges(ranges: &[ColumnRange]) -> String {
    if ranges.is_empty() {
        return "-".to_string();
    }
    ranges
        .iter()
        .map(|r| format!("{}-{}", r.start, r.end))
        .collect::<Vec<_>>()
        .join(", ")
}
