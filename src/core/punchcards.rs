use crate::core::encoding::{CellMask, EncodeError, PunchEncoding};
use std::fmt::{self, Write};

const COLS: usize = 80;
const ROW_LABELS: [&str; 12] = [
    "12", "11", " 0", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8", " 9",
];
const ROW_BIT_ORDER: [usize; 12] = [11, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
const BLANK_CARD: &str =
    "                                                                                ";

/// In-memory representation of a single punch card column-by-column.
#[derive(Debug, Clone)]
pub struct PunchCard {
    columns: [CellMask; COLS],
    text: [char; COLS],
}

impl PunchCard {
    pub fn from_str<E: PunchEncoding + ?Sized>(enc: &E, s: &str) -> Result<Self, EncodeError> {
        let mut columns = [CellMask(0); COLS];
        let mut text = [' '; COLS];
        for (idx, ch) in s.chars().take(COLS).enumerate() {
            columns[idx] = enc.encode_char(ch)?;
            text[idx] = ch;
        }
        Ok(Self { columns, text })
    }

    pub fn with_sequence<E: PunchEncoding + ?Sized>(
        mut self,
        enc: &E,
        seq: usize,
    ) -> Result<Self, EncodeError> {
        let seq_repr = format!("{:>9}", seq);
        let start = COLS - seq_repr.len();
        for (offset, ch) in seq_repr.chars().enumerate() {
            let idx = start + offset;
            if self.text[idx] != ' ' {
                continue;
            }
            self.text[idx] = ch;
            self.columns[idx] = enc.encode_char(ch)?;
        }
        Ok(self)
    }

    pub fn render(&self, style: RenderStyle) -> String {
        match style {
            RenderStyle::AsciiX => self.render_ascii('X', ' '),
            RenderStyle::Ascii01 => self.render_ascii('1', '0'),
        }
    }

    pub fn columns(&self) -> &[CellMask; COLS] {
        &self.columns
    }

    pub fn text(&self) -> &[char; COLS] {
        &self.text
    }

    fn render_ascii(&self, mark: char, blank: char) -> String {
        let mut out = String::with_capacity(16 * COLS);
        writeln!(&mut out, "IBM 5081 (80 cols) [IBM029]").unwrap();
        writeln!(&mut out, "     {}", ruler_line()).unwrap();
        write!(&mut out, "     ").unwrap();
        out.extend(self.text);
        writeln!(&mut out).unwrap();
        let separator = "-".repeat(COLS);
        writeln!(&mut out, "     {}", separator).unwrap();
        for (row_index, label) in ROW_LABELS.iter().enumerate() {
            write!(&mut out, "{:>3} |", label).unwrap();
            let bit = ROW_BIT_ORDER[row_index];
            for cell in &self.columns {
                let filled = (cell.0 >> bit) & 1 == 1;
                out.push(if filled { mark } else { blank });
            }
            writeln!(&mut out, "|").unwrap();
        }
        writeln!(&mut out, "     {}", separator).unwrap();
        out
    }
}

fn ruler_line() -> String {
    let mut ruler = String::with_capacity(COLS);
    for col in 1..=COLS {
        if col % 10 == 0 {
            let digit = ((col / 10) % 10) as u8;
            ruler.push(char::from(b'0' + digit));
        } else {
            ruler.push('.');
        }
    }
    ruler
}

/// Logical collection of punch cards.
#[derive(Debug, Clone)]
pub struct CardDeck {
    pub cards: Vec<PunchCard>,
}

impl CardDeck {
    pub fn from_text<E: PunchEncoding + ?Sized>(
        enc: &E,
        text: &str,
        with_seq_numbers: bool,
    ) -> anyhow::Result<Self> {
        let mut cards = Vec::new();
        let mut seq = 1usize;
        for line in text.lines() {
            Self::split_line(enc, line, with_seq_numbers, &mut seq, &mut cards)?;
        }
        if cards.is_empty() {
            let mut blank = PunchCard::from_str(enc, BLANK_CARD)?;
            if with_seq_numbers {
                blank = blank.with_sequence(enc, 1)?;
            }
            cards.push(blank);
        }
        Ok(Self { cards })
    }

    fn split_line<E: PunchEncoding + ?Sized>(
        enc: &E,
        line: &str,
        with_seq_numbers: bool,
        seq: &mut usize,
        out: &mut Vec<PunchCard>,
    ) -> anyhow::Result<()> {
        let mut buffer = String::with_capacity(COLS);
        let mut count = 0usize;
        for ch in line.chars() {
            buffer.push(ch);
            count += 1;
            if count == COLS {
                Self::push_card(enc, &buffer, with_seq_numbers, seq, out)?;
                buffer.clear();
                count = 0;
            }
        }

        if count > 0 {
            while count < COLS {
                buffer.push(' ');
                count += 1;
            }
            Self::push_card(enc, &buffer, with_seq_numbers, seq, out)?;
        } else if line.is_empty() {
            Self::push_card(enc, BLANK_CARD, with_seq_numbers, seq, out)?;
        }
        Ok(())
    }

    fn push_card<E: PunchEncoding + ?Sized>(
        enc: &E,
        text: &str,
        with_seq_numbers: bool,
        seq: &mut usize,
        out: &mut Vec<PunchCard>,
    ) -> anyhow::Result<()> {
        let mut card = PunchCard::from_str(enc, text)?;
        if with_seq_numbers {
            card = card.with_sequence(enc, *seq)?;
            *seq += 1;
        }
        out.push(card);
        Ok(())
    }

    pub fn render(&self, style: RenderStyle) -> String {
        let mut out = String::new();
        for card in &self.cards {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&card.render(style));
        }
        out
    }
}

/// ASCII rendering styles.
#[derive(Debug, Clone, Copy)]
pub enum RenderStyle {
    AsciiX,
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
