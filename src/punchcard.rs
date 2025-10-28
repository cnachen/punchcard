use crate::encoding::{CellMask, EncodeError, PunchEncoding};
use std::fmt::{self, Write};

const COLS: usize = 80;
const _ROWS: usize = 12; // 12, 11, 0..9 -> total 12 rows

#[derive(Debug, Clone)]
pub struct PunchCard {
    pub cols: [CellMask; COLS],
    pub raw_text: String,
}

impl PunchCard {
    pub fn new() -> Self {
        Self {
            cols: [CellMask(0); COLS],
            raw_text: String::new(),
        }
    }

    pub fn from_str<E: PunchEncoding>(enc: &E, s: &str) -> Result<Self, EncodeError> {
        let mut card = Self::new();
        card.raw_text = s.chars().take(COLS).collect();
        for (i, ch) in s.chars().take(COLS).enumerate() {
            card.cols[i] = enc.encode_char(ch)?;
        }
        Ok(card)
    }

    /// Write a sequence number in the lower-right corner (columns 72..80, a common practice).
    /// Automatically truncated if it exceeds 9 digits.
    pub fn with_sequence(mut self, seq: usize) -> Self {
        let seq_str = format!("{:>9}", seq);
        // Place sequence number in columns 72..80 (indices 71..80) — a common deck convention, can be customized
        let start = COLS.saturating_sub(seq_str.len());
        for (i, ch) in seq_str.chars().enumerate() {
            // Unknown characters will result in blank punches
            if let Ok(mask) = crate::encoding::Ibm029Encoder::new().encode_char(ch) {
                self.cols[start + i] = mask;
            }
        }
        self
    }

    pub fn render(&self, style: RenderStyle) -> String {
        match style {
            RenderStyle::AsciiX => self.render_ascii('X', ' '),
            RenderStyle::Ascii01 => self.render_ascii('1', '0'),
        }
    }

    fn render_ascii(&self, mark: char, blank: char) -> String {
        // Refer to README illustration: row order is 12, 11, 0..9, with a ruler line at the top
        let mut out = String::new();
        let ruler =
            ".........1.........2.........3.........4.........5.........6.........7.........8";
        writeln!(&mut out, "IBM 5081 (80 cols) [{}]", "IBM029").ok();
        writeln!(&mut out, "     {}", &ruler[..80]).ok();

        write!(&mut out, "     ").ok();
        for ch in self
            .raw_text
            .chars()
            .chain(std::iter::repeat(' '))
            .take(COLS)
        {
            out.push(ch);
        }

        writeln!(&mut out, "").ok();

        writeln!(&mut out, "     {}", "-".repeat(80)).ok();

        // Row labels: 12/11/0..9
        let labels = [
            "12", "11", " 0", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8", " 9",
        ];
        for (r, label) in labels.iter().enumerate() {
            write!(&mut out, "{:>3} |", label).ok();
            for c in 0..COLS {
                let bit = match r {
                    0 => 11,    // row 12
                    1 => 10,    // row 11
                    _ => r - 2, // rows 0..9
                };
                let filled = (self.cols[c].0 >> bit) & 1 == 1;
                out.push(if filled { mark } else { blank });
            }
            writeln!(&mut out, "|").ok();
        }
        writeln!(&mut out, "     {}", "-".repeat(80)).ok();
        out
    }
}

#[derive(Debug, Clone)]
pub struct CardDeck {
    pub cards: Vec<PunchCard>,
}

impl CardDeck {
    pub fn from_text<E: PunchEncoding>(
        enc: &E,
        text: &str,
        with_seq_numbers: bool,
    ) -> anyhow::Result<Self> {
        let mut cards = Vec::new();
        let mut seq = 1usize;
        for line in text.lines() {
            // Each line may exceed 80 columns; split every 80 characters
            let mut buf = String::new();
            for ch in line.chars() {
                buf.push(ch);
                if buf.chars().count() == 80 {
                    let mut card = PunchCard::from_str(enc, &buf)?;
                    if with_seq_numbers {
                        card = card.with_sequence(seq);
                    }
                    cards.push(card);
                    seq += 1;
                    buf.clear();
                }
            }
            // Handle any remaining characters (<80) as a single card
            if !buf.is_empty() {
                let mut padded = buf;
                // Pad with spaces on the right
                while padded.chars().count() < 80 {
                    padded.push(' ');
                }
                let mut card = PunchCard::from_str(enc, &padded)?;
                if with_seq_numbers {
                    card = card.with_sequence(seq);
                }
                cards.push(card);
                seq += 1;
            } else if line.is_empty() {
                // Empty line → one blank card
                let mut card = PunchCard::from_str(enc, &" ".repeat(80))?;
                if with_seq_numbers {
                    card = card.with_sequence(seq);
                }
                cards.push(card);
                seq += 1;
            }
        }
        if cards.is_empty() {
            // No lines in input → produce one blank card
            let mut card = PunchCard::from_str(enc, &" ".repeat(80))?;
            if with_seq_numbers {
                card = card.with_sequence(1);
            }
            cards.push(card);
        }
        Ok(Self { cards })
    }

    pub fn render(&self, style: RenderStyle) -> String {
        let mut s = String::new();
        for (i, c) in self.cards.iter().enumerate() {
            if i > 0 {
                s.push_str("\n");
            }
            s.push_str(&c.render(style.clone()));
        }
        s
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderStyle {
    /// Use 'X' to mark punched holes
    AsciiX,
    /// Use '1'/'0' to mark punched/unpunched
    Ascii01,
}

impl fmt::Display for RenderStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderStyle::AsciiX => write!(f, "ascii-x"),
            RenderStyle::Ascii01 => write!(f, "ascii-01"),
        }
    }
}
