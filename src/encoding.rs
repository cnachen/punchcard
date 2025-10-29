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

const IBM029_TABLE: &[(char, &str)] = &[
    ('&', "100000000000"),
    ('-', "010000000000"),
    ('0', "001000000000"),
    ('1', "000100000000"),
    ('2', "000010000000"),
    ('3', "000001000000"),
    ('4', "000000100000"),
    ('5', "000000010000"),
    ('6', "000000001000"),
    ('7', "000000000100"),
    ('8', "000000000010"),
    ('9', "000000000001"),
    ('A', "100100000000"),
    ('B', "100010000000"),
    ('C', "100001000000"),
    ('D', "100000100000"),
    ('E', "100000010000"),
    ('F', "100000001000"),
    ('G', "100000000100"),
    ('H', "100000000010"),
    ('I', "100000000001"),
    ('J', "010100000000"),
    ('K', "010010000000"),
    ('L', "010001000000"),
    ('M', "010000100000"),
    ('N', "010000010000"),
    ('O', "010000001000"),
    ('P', "010000000100"),
    ('Q', "010000000010"),
    ('R', "010000000001"),
    ('/', "001100000000"),
    ('S', "001010000000"),
    ('T', "001001000000"),
    ('U', "001000100000"),
    ('V', "001000010000"),
    ('W', "001000001000"),
    ('X', "001000000100"),
    ('Y', "001000000010"),
    ('Z', "001000000001"),
    (':', "000010000010"),
    ('#', "000001000010"),
    ('@', "000000100010"),
    ('\'', "000000010010"),
    ('=', "000000001010"),
    ('\"', "000000000110"),
    ('¢', "100010000010"),
    ('.', "100001000010"),
    ('<', "100000100010"),
    ('(', "100000010010"),
    ('+', "100000001010"),
    ('|', "100000000110"),
    ('!', "010010000010"),
    ('$', "010001000010"),
    ('*', "010000100010"),
    (')', "010000010010"),
    (';', "010000001010"),
    ('¬', "010000000110"),
    (' ', "000000000000"),
    (',', "001001000010"),
    ('%', "001000100010"),
    ('_', "001000010010"),
    ('>', "001000001010"),
    ('?', "001000000110"),
];

/// Summary of IBM 029 (Hollerith) encoding rules:
/// - Each column can punch any of 12 rows (12, 11, 0–9).
/// - Digits, letters, and special characters map to unique hole combinations.
/// - The table above reproduces the original 029 keypunch chart.
#[derive(Default)]
pub struct Ibm029Encoder {
    map: HashMap<char, CellMask>,
}

impl Ibm029Encoder {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        for (ch, bits) in IBM029_TABLE {
            m.insert(*ch, mask_from_bits(bits));
        }
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

const ROW_BIT_ORDER: [usize; 12] = [11, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

fn mask_from_bits(bits: &str) -> CellMask {
    assert_eq!(
        bits.len(),
        ROW_BIT_ORDER.len(),
        "IBM029 bit strings must have 12 characters"
    );
    let mut value = 0u16;
    for (idx, ch) in bits.chars().enumerate() {
        match ch {
            '0' => {}
            '1' => value |= 1u16 << ROW_BIT_ORDER[idx],
            other => panic!("unexpected character '{}' in IBM029 table", other),
        }
    }
    CellMask(value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn no_duplicate_hole_patterns() {
        let enc = Ibm029Encoder::new();
        let mut seen: HashMap<u16, char> = HashMap::new();
        let mut chars: HashSet<char> = VALID_SET.chars().collect();
        chars.insert(' ');
        for ch in chars {
            let mask = enc.encode_char(ch).unwrap();
            if let Some(prev) = seen.insert(mask.0, ch) {
                panic!("characters '{}' and '{}' share the same punches", prev, ch);
            }
        }
    }
}
