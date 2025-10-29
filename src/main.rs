use anyhow::Result;
use clap::{ArgGroup, Parser};
use punchcard::{Ibm029Encoder, PunchEncoding, RenderStyle, encode_text_to_deck};
use std::fs;
use std::io::{self, Read};

/// Rust rewrite of barrettotte/punchit (IBM 5081 / IBM 029)
#[derive(Parser, Debug)]
#[command(
    name = "punchit",
    version,
    about = "Convert text to IBM 5081 (80-col) punch cards (IBM029)",
    group(ArgGroup::new("input").required(false).args(&["file"]))
)]
struct Cli {
    /// Input file (default: read from stdin)
    file: Option<String>,

    /// Render ASCII punch pattern to stdout
    #[arg(long)]
    render: bool,

    /// Add 9-digit sequence numbers at right end
    #[arg(long, short = 's')]
    seq: bool,

    /// Render style: ascii-x | ascii-01
    #[arg(long, value_parser = ["ascii-x", "ascii-01"], default_value = "ascii-x")]
    style: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = if let Some(f) = &cli.file {
        fs::read_to_string(f)?
    } else {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        s
    };

    let enc = Ibm029Encoder::new();
    let deck = encode_text_to_deck(&enc, &input, cli.seq)?;

    if cli.render {
        let style = match cli.style.as_str() {
            "ascii-01" => RenderStyle::Ascii01,
            _ => RenderStyle::AsciiX,
        };
        println!("{}", deck.render(style));
    } else {
        println!(
            "Encoded with {}: {} cards (80 cols each)",
            enc.name(),
            deck.cards.len()
        );
    }

    Ok(())
}
