use std::collections::HashMap;
use thiserror::Error;

/// There are 12 rows in total: 12, 11, and 0..9.  
/// Each column’s punched holes are represented as a bitmask in a `u16`.  
/// Bit meaning (LSB → MSB): bit0 = row 0, bit1 = row 1, ..., bit9 = row 9, bit10 = row 11, bit11 = row 12.  
/// This layout allows easy row-by-row rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMask(pub u16);

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("unsupported character: '{0}' (U+{1:04X})")]
    Unsupported(char, u32),
}

impl std::ops::BitOr for CellMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        CellMask(self.0 | rhs.0)
    }
}

pub trait PunchEncoding {
    fn name(&self) -> &'static str;
    fn encode_char(&self, ch: char) -> Result<CellMask, EncodeError>;
    fn is_supported(&self, ch: char) -> bool {
        self.encode_char(ch).is_ok()
    }
}

/// Valid character set (source: original project README)
pub const VALID_SET: &str = "&-0123456789ABCDEFGHIJKLMNOPQR/STUVWXYZ:#@'=\"¢.<(+|!$*);¬ ,%_>?";

/// For practical use, only **letters / digits / space** are fully implemented.  
/// Other symbols belong to IBM 029 “special combinations” — these mappings are numerous,  
/// so they are marked as TODO for now. Unsupported characters will trigger an error  
/// to prevent silent misencoding.
///
/// Summary of common IBM 029 (Hollerith) encoding rules:  
/// - Digits 0–9: punch the corresponding row (0–9)  
/// - A–I: 12 + 1–9; J–R: 11 + 1–9; S–Z: 0 + 2–9 (Z = 0+9)  
/// - Space: no punches  
///
/// Reference: Original repository documentation and general punch card conventions.
#[derive(Default)]
pub struct Ibm029Encoder {
    map: HashMap<char, CellMask>,
}

impl Ibm029Encoder {
    pub fn new() -> Self {
        let mut m = HashMap::new();

        // Space = no punches
        m.insert(' ', CellMask(0));

        // Digits 0..9 → rows 0..9
        for d in '0'..='9' {
            let row = if d == '0' { 0 } else { d as u8 - b'0' } as usize;
            m.insert(d, row_mask(row));
        }

        // A–I : 12 + 1..9
        for (i, ch) in ('A'..='I').enumerate() {
            m.insert(ch, zone12() | row_mask(i + 1));
        }
        // J–R : 11 + 1..9
        for (i, ch) in ('J'..='R').enumerate() {
            m.insert(ch, zone11() | row_mask(i + 1));
        }
        // S–Z : 0 + 2..9  (Z = 0 + 9)
        // S(2)..Y(8), Z(9)
        let mut r = 2usize;
        for ch in 'S'..='Y' {
            m.insert(ch, row_mask(0) | row_mask(r));
            r += 1;
        }
        m.insert('Z', row_mask(0) | row_mask(9));

        // Symbols and special characters according to the IBM 029 manual
        // (also cross-checked with punchit/encoding.go)
        // zone12 = bit 11, zone11 = bit 10, row_mask(n) corresponds to row n (0–9)
        m.insert('&', zone12() | row_mask(3)); // 12-3
        m.insert('-', zone11() | row_mask(8)); // 11-8
        m.insert('/', zone11() | row_mask(0)); // 11-0
        m.insert(':', zone12() | row_mask(8)); // 12-8
        m.insert('#', zone12() | row_mask(6)); // 12-6
        m.insert('@', zone12() | row_mask(1)); // 12-1
        m.insert('\'', zone11() | row_mask(3)); // 11-3
        m.insert('=', zone11() | row_mask(6)); // 11-6
        m.insert('"', zone12() | row_mask(4)); // 12-4
        m.insert('¢', zone11() | row_mask(2)); // 11-2
        m.insert('.', row_mask(9)); // 9
        m.insert('<', row_mask(8)); // 8
        m.insert('(', row_mask(7)); // 7
        m.insert('+', row_mask(6)); // 6
        m.insert('|', row_mask(5)); // 5
        m.insert('!', row_mask(4)); // 4
        m.insert('$', row_mask(3)); // 3
        m.insert('*', row_mask(2)); // 2
        m.insert(')', row_mask(1)); // 1
        m.insert(';', zone11() | row_mask(9)); // 11-9
        m.insert('¬', zone11() | row_mask(4)); // 11-4
        m.insert(',', zone12() | row_mask(9)); // 12-9
        m.insert('%', zone11() | row_mask(5)); // 11-5
        m.insert('_', zone11() | row_mask(7)); // 11-7
        m.insert('>', zone12() | row_mask(7)); // 12-7
        m.insert('?', zone12() | row_mask(5)); // 12-5

        Self { map: m }
    }
}

impl PunchEncoding for Ibm029Encoder {
    fn name(&self) -> &'static str {
        "IBM029"
    }

    fn encode_char(&self, ch: char) -> Result<CellMask, EncodeError> {
        let up = if ch.is_ascii_lowercase() {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        self.map
            .get(&up)
            .copied()
            .ok_or(EncodeError::Unsupported(ch, ch as u32))
    }
}

/// Bit utilities: rows 0..9
fn row_mask(row: usize) -> CellMask {
    // bit0..bit9 represent rows 0..9 respectively
    CellMask(1u16 << row)
}
fn zone11() -> CellMask {
    CellMask(1u16 << 10)
}
fn zone12() -> CellMask {
    CellMask(1u16 << 11)
}

/// Public helper: checks if a character belongs to the original valid set
pub struct ValidChar;
impl ValidChar {
    pub fn in_original_set(ch: char) -> bool {
        // Ignore case
        let up = if ch.is_ascii_lowercase() {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        up == ' ' || VALID_SET.contains(up)
    }
}
