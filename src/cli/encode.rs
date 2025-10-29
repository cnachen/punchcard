//! Encoding helpers (`punch encode ...`).

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use punchcard::{Ibm029Encoder, RenderStyle, encode_text_to_deck};

use crate::cli::utils::read_text_arg;

/// Encode subcommands.
#[derive(Subcommand, Debug)]
pub enum EncodeCommand {
    /// Encode text into punch card deck.
    Text(EncodeTextArgs),
}

/// Arguments for `punch encode text`.
#[derive(Args, Debug)]
pub struct EncodeTextArgs {
    /// Input text (falls back to stdin if omitted).
    #[arg(long)]
    pub text: Option<String>,
    /// Read input from file (`-` for stdin).
    #[arg(long = "from")]
    pub from: Option<PathBuf>,
    /// Render ASCII representation.
    #[arg(long)]
    pub render: bool,
}

/// Execute an encode command.
pub fn handle(command: EncodeCommand) -> Result<()> {
    match command {
        EncodeCommand::Text(args) => text(args),
    }
}

fn text(args: EncodeTextArgs) -> Result<()> {
    let text = read_text_arg(args.text.clone(), args.from.clone())?;
    let encoder = Ibm029Encoder::new();
    let deck = encode_text_to_deck(&encoder, &text, true)?;
    if args.render {
        println!("{}", deck.render(RenderStyle::AsciiX));
    } else {
        println!(
            "Encoded {} columns into {} cards",
            text.len(),
            deck.cards.len()
        );
    }
    Ok(())
}
