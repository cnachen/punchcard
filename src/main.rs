use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use punchcard::{Ibm029Encoder, PunchEncoding, RenderStyle, encode_text_to_deck};
use std::fs;
use std::io::{self, Read};

/// Rust rewrite of barrettotte/punchit (IBM 5081 / IBM 029)
#[derive(Parser, Debug)]
#[command(
    name = "punchit",
    version,
    about = "Convert text to IBM 5081 (80-col) punch cards (IBM029)"
)]
struct Cli {
    /// Input file (default: read from stdin)
    #[arg(short, long)]
    file: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Render a textual card visualization
    #[command(alias = "r")]
    Render(RenderArgs),
}

#[derive(Args, Debug)]
struct RenderArgs {
    /// Append right-aligned 9-digit sequence numbers (cols 72-80)
    #[arg(short = 's', long = "seq")]
    seq: bool,

    /// Rendering style to use
    #[arg(short = 'S', long = "style", value_enum, default_value_t = RenderStyleArg::AsciiX)]
    style: RenderStyleArg,
}

#[derive(Clone, Debug, ValueEnum)]
enum RenderStyleArg {
    #[value(name = "ascii-x")]
    AsciiX,
    #[value(name = "ascii-01")]
    Ascii01,
}

impl From<RenderStyleArg> for RenderStyle {
    fn from(value: RenderStyleArg) -> Self {
        match value {
            RenderStyleArg::AsciiX => RenderStyle::AsciiX,
            RenderStyleArg::Ascii01 => RenderStyle::Ascii01,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = match &cli.file {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            s
        }
    };

    let enc = Ibm029Encoder::new();

    match cli.command {
        Some(Command::Render(render)) => {
            let deck = encode_text_to_deck(&enc, &input, render.seq)?;
            println!("{}", deck.render(render.style.into()));
        }
        None => {
            let deck = encode_text_to_deck(&enc, &input, false)?;
            println!(
                "Encoded with {}: {} cards (80 cols each)",
                enc.name(),
                deck.cards.len()
            );
        }
    }

    Ok(())
}
