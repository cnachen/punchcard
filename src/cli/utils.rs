//! Convenience helpers shared across command handlers.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use punchcard::{ColumnRange, Deck};

/// Resolve plain-text input for commands that accept either inline strings or files.
pub fn read_text_arg(text: Option<String>, from: Option<PathBuf>) -> Result<String> {
    if let Some(t) = text {
        return Ok(t);
    }
    if let Some(path) = from {
        if path.as_os_str() == "-" {
            return read_stdin();
        }
        return fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()));
    }
    read_stdin()
}

/// Read the entire stdin stream into memory.
pub fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("failed to read from stdin")?;
    Ok(buffer)
}

/// Persist a string either to a file or stdout when `-` is provided.
pub fn write_output(path: &Path, content: &str) -> Result<()> {
    if path.as_os_str() == "-" {
        io::stdout().write_all(content.as_bytes())?;
        return Ok(());
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

/// Clap-friendly column range parser for strings like `73-80`.
pub fn parse_column_range(input: &str) -> Result<ColumnRange, String> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 2 {
        return Err("column range must be START-END".to_string());
    }
    let start: usize = parts[0]
        .parse()
        .map_err(|_| "start column must be a number".to_string())?;
    let end: usize = parts[1]
        .parse()
        .map_err(|_| "end column must be a number".to_string())?;
    ColumnRange::new(start, end).map_err(|err| err.to_string())
}

/// Expand range expressions such as `1..10,25,40..$` into zero-based card indices.
pub fn parse_range_expression(expr: &str, deck_len: usize) -> Result<Vec<usize>> {
    if expr.trim().is_empty() {
        return Err(anyhow!("range expression cannot be empty"));
    }
    let mut indices: Vec<usize> = Vec::new();
    for part in expr.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_raw, end_raw)) = part.split_once("..") {
            let start = parse_range_bound(start_raw.trim(), deck_len)?;
            let end = parse_range_bound(end_raw.trim(), deck_len)?;
            if start > end {
                return Err(anyhow!("range {}..{} is invalid", start, end));
            }
            for value in start..=end {
                indices.push(value - 1);
            }
        } else {
            let value = parse_range_bound(part, deck_len)?;
            indices.push(value - 1);
        }
    }
    if indices.is_empty() {
        return Err(anyhow!("no indices resolved from '{}'", expr));
    }
    let mut unique: Vec<usize> = Vec::new();
    for idx in indices {
        if idx >= deck_len {
            return Err(anyhow!(
                "card index {} out of range 1..{}",
                idx + 1,
                deck_len
            ));
        }
        if !unique.contains(&idx) {
            unique.push(idx);
        }
    }
    Ok(unique)
}

fn parse_range_bound(token: &str, deck_len: usize) -> Result<usize> {
    if token.is_empty() {
        return Err(anyhow!("range bound cannot be empty"));
    }
    if token == "$" {
        if deck_len == 0 {
            return Err(anyhow!("deck is empty; '$' is undefined"));
        }
        return Ok(deck_len);
    }
    let value: usize = token
        .parse()
        .map_err(|_| anyhow!("range bound '{}' is not a number", token))?;
    if value == 0 {
        return Err(anyhow!("card indices are 1-based"));
    }
    Ok(value)
}

/// Split arbitrary input into 80-column padded card strings.
pub fn split_lines_fixed(input: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in input.lines() {
        let mut chars: Vec<char> = raw.chars().collect();
        if chars.len() > 80 {
            chars.truncate(80);
        }
        while chars.len() < 80 {
            chars.push(' ');
        }
        lines.push(chars.into_iter().collect());
    }
    if lines.is_empty() {
        lines.push(" ".repeat(80));
    }
    lines
}

/// Location for storing the verification baseline for a given deck.
pub fn verify_snapshot_path(deck: &Path) -> PathBuf {
    let mut path = deck.to_path_buf();
    path.set_extension("verify.base");
    path
}

/// Location for storing the latest verification diff for a deck.
pub fn verify_diff_path(deck: &Path) -> PathBuf {
    let mut path = deck.to_path_buf();
    path.set_extension("verify.diff");
    path
}

/// Produce a human-readable diff, respecting optional masked column ranges.
pub fn diff_text(expected: &str, actual: &str, mask: &[ColumnRange]) -> (String, bool) {
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let max = exp_lines.len().max(act_lines.len());
    let mut output = String::new();
    let mut changed = false;
    for i in 0..max {
        let exp = exp_lines.get(i).copied().unwrap_or("");
        let act = act_lines.get(i).copied().unwrap_or("");
        if !lines_match_with_mask(exp, act, mask) {
            changed = true;
            output.push_str(&format!("line {:>4}:\n", i + 1));
            output.push_str(&format!("  expected |{}|\n", exp));
            output.push_str(&format!("  actual   |{}|\n", act));
        }
    }
    if !changed {
        output.push_str("verification passed: no differences\n");
    }
    (output, changed)
}

fn lines_match_with_mask(expected: &str, actual: &str, mask: &[ColumnRange]) -> bool {
    if expected == actual && mask.is_empty() {
        return true;
    }
    let mut exp_chars: Vec<char> = expected.chars().collect();
    let mut act_chars: Vec<char> = actual.chars().collect();
    let required_len = mask.iter().map(|r| r.end).max().unwrap_or(0);
    while exp_chars.len() < required_len {
        exp_chars.push(' ');
    }
    while act_chars.len() < required_len {
        act_chars.push(' ');
    }
    for range in mask {
        for col in range.start..=range.end {
            let idx = col - 1;
            if idx < exp_chars.len() {
                exp_chars[idx] = '_';
            }
            if idx < act_chars.len() {
                act_chars[idx] = '_';
            }
        }
    }
    exp_chars == act_chars
}

/// Load a deck file, attaching path context to any error.
pub fn load_deck(path: &Path) -> Result<Deck> {
    Deck::load(path).with_context(|| format!("failed to read deck {}", path.display()))
}
