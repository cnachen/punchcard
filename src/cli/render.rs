//! Rendering commands (`punch render ...`).

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};
use punchcard::{render_card_image, ImageRenderOptions, Ibm029Encoder};

use crate::cli::utils::load_deck;

use crate::cli::common::{CardImageStyleArg, PageLayoutArg, RenderStyleArg};
use crate::cli::utils::write_output;

/// Available render subcommands.
#[derive(Subcommand, Debug)]
pub enum RenderCommand {
    /// Render rich PNG images of the card faces.
    Image(RenderImageArgs),
    /// Produce interpreter-style listing.
    Interpret(RenderInterpretArgs),
    /// Emit a card-by-card textual listing.
    Listing(RenderListingArgs),
}

/// Args for `punch render image`.
#[derive(Args, Debug)]
pub struct RenderImageArgs {
    /// Deck file to render.
    pub deck: PathBuf,
    /// Output file or directory for generated PNGs.
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    /// Visual style applied to the card face.
    #[arg(long, default_value_t = CardImageStyleArg::Interpreter, value_enum)]
    pub style: CardImageStyleArg,
    /// Output page layout.
    #[arg(long = "pagesize", default_value_t = PageLayoutArg::Card, value_enum)]
    pub pagesize: PageLayoutArg,
    /// Dots per inch used when rasterising.
    #[arg(long, default_value_t = 300)]
    pub dpi: u32,
}

/// Args for `punch render interpret`.
#[derive(Args, Debug)]
pub struct RenderInterpretArgs {
    /// Deck file to render.
    pub deck: PathBuf,
    /// Output file (`-` for stdout).
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    /// Rendering style.
    #[arg(long, default_value_t = RenderStyleArg::AsciiX, value_enum)]
    pub style: RenderStyleArg,
}

/// Args for `punch render listing`.
#[derive(Args, Debug)]
pub struct RenderListingArgs {
    /// Deck file to render.
    pub deck: PathBuf,
    /// Output file (`-` for stdout)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    /// Rendering style for punch visualization.
    #[arg(long, default_value_t = RenderStyleArg::AsciiX, value_enum)]
    pub style: RenderStyleArg,
}

/// Execute a render command.
pub fn handle(command: RenderCommand) -> Result<()> {
    match command {
        RenderCommand::Image(args) => image(args),
        RenderCommand::Interpret(args) => interpret(args),
        RenderCommand::Listing(args) => listing(args),
    }
}

fn image(args: RenderImageArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    let dpi = args.dpi.clamp(72, 1200);
    let options = ImageRenderOptions {
        style: args.style.into(),
        dpi,
        layout: args.pagesize.into(),
    };

    let output_path = args.output;
    let is_single_file_target = output_path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false);

    if deck.cards.len() > 1 && is_single_file_target {
        return Err(anyhow!(
            "output path must be a directory when rendering multiple cards"
        ));
    }

    if is_single_file_target {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create output directory {}", parent.display())
                })?;
            }
        }
    } else {
        fs::create_dir_all(&output_path).with_context(|| {
            format!("failed to create output directory {}", output_path.display())
        })?;
    }

    let encoder = Ibm029Encoder::new();
    let punch_deck = deck
        .to_punch_deck(&encoder)
        .context("failed to render deck with IBM029 encoder")?;

    for (idx, card) in punch_deck.cards.iter().enumerate() {
        let target_path = if is_single_file_target {
            output_path.clone()
        } else {
            output_path.join(format!("card_{:04}.png", idx + 1))
        };
        let image = render_card_image(card, &options)?;
        image
            .save(&target_path)
            .with_context(|| format!("failed to write {}", target_path.display()))?;
    }

    if is_single_file_target {
        println!(
            "Rendered card image to {} at {} DPI",
            output_path.display(),
            dpi
        );
    } else {
        println!(
            "Rendered {} card image(s) to {} at {} DPI",
            deck.cards.len(),
            output_path.display(),
            dpi
        );
    }
    Ok(())
}

fn interpret(args: RenderInterpretArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    let encoder = Ibm029Encoder::new();
    let punch_deck = deck
        .to_punch_deck(&encoder)
        .context("failed to render deck with IBM029 encoder")?;
    let mut output = String::new();
    for (idx, card) in punch_deck.cards.iter().enumerate() {
        if idx > 0 {
            output.push('\n');
        }
        output.push_str(&card.render(args.style.into()));
    }
    match args.output {
        Some(path) => {
            write_output(&path, &output)?;
            println!(
                "Wrote interpreted listing for {} to {}",
                args.deck.display(),
                path.display()
            );
        }
        None => {
            print!("{}", output);
        }
    }
    Ok(())
}

fn listing(args: RenderListingArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    let encoder = Ibm029Encoder::new();
    let punch_deck = deck
        .to_punch_deck(&encoder)
        .context("failed to render deck with IBM029 encoder")?;
    let mut output = String::new();
    for (idx, (record, card)) in deck.cards.iter().zip(punch_deck.cards.iter()).enumerate() {
        if idx > 0 {
            output.push_str("\n\n");
        }
        let label = record
            .seq
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(none)".to_string());
        output.push_str(&format!(
            "Card {:>4} | seq {} | type {:?}\n",
            idx + 1,
            label,
            record.card_type
        ));
        if let Some(note) = record.meta.note.as_ref() {
            output.push_str(&format!("Note: {}\n", note));
        }
        if let Some(color) = record.meta.color.as_ref() {
            output.push_str(&format!("Color: {}\n", color));
        }
        let text = record.text.as_deref().unwrap_or("(stored punches)");
        output.push_str("Text:\n");
        output.push_str(text);
        output.push('\n');
        output.push_str("Punches:\n");
        output.push_str(&card.render(args.style.into()));
    }
    match args.output {
        Some(path) => {
            write_output(&path, &output)?;
            println!(
                "Wrote listing for {} to {}",
                args.deck.display(),
                path.display()
            );
        }
        None => {
            print!("{}", output);
        }
    }
    Ok(())
}
