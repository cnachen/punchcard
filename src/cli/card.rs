//! Card-level operations (`punch card ...`).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand};
use punchcard::{
    CardMeta, CardRecord, CardType, EncodingKind, Ibm029Encoder, RenderStyle, TemplateRegistry,
};

use crate::cli::common::CardTypeArg;
use crate::cli::utils::{load_deck, read_stdin, read_text_arg, split_lines_fixed};

/// Supported `punch card` subcommands.
#[derive(Subcommand, Debug)]
pub enum CardCommand {
    /// Append or insert cards using raw text.
    Add(CardAddArgs),
    /// Type cards interactively from stdin.
    Type(CardTypeArgs),
    /// Replace an existing card by index.
    Replace(CardReplaceArgs),
    /// Show a card with metadata.
    Show(CardShowArgs),
    /// Insert a separator/comment card.
    Patch(CardPatchArgs),
}

/// Arguments for `punch card add`.
#[derive(Args, Debug)]
pub struct CardAddArgs {
    /// Deck file to modify.
    pub deck: PathBuf,
    /// Direct text for the card (80 chars or fewer).
    #[arg(long)]
    pub text: Option<String>,
    /// Load card text from file (`-` for stdin).
    #[arg(long = "from")]
    pub from: Option<PathBuf>,
    /// Apply template defaults.
    #[arg(long)]
    pub template: Option<String>,
    /// Explicit card type.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    pub card_type: CardTypeArg,
    /// Optional human note.
    #[arg(long)]
    pub note: Option<String>,
    /// Optional color hint.
    #[arg(long)]
    pub color: Option<String>,
    /// Insert at 1-based position (defaults to append).
    #[arg(long)]
    pub position: Option<usize>,
}

/// Arguments for `punch card type`.
#[derive(Args, Debug)]
pub struct CardTypeArgs {
    /// Deck file to modify.
    pub deck: PathBuf,
    /// Apply template defaults during typing.
    #[arg(long)]
    pub template: Option<String>,
    /// Explicit card type.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    pub card_type: CardTypeArg,
    /// Optional human note applied to all typed cards.
    #[arg(long)]
    pub note: Option<String>,
    /// Optional color hint.
    #[arg(long)]
    pub color: Option<String>,
}

/// Arguments for `punch card replace`.
#[derive(Args, Debug)]
pub struct CardReplaceArgs {
    /// Deck file to modify.
    pub deck: PathBuf,
    /// 1-based index of the card to replace.
    #[arg(short = 'i', long = "index")]
    pub index: usize,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long = "from")]
    pub from: Option<PathBuf>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub color: Option<String>,
    #[arg(long = "type", value_enum)]
    pub card_type: Option<CardTypeArg>,
}

/// Arguments for `punch card show`.
#[derive(Args, Debug)]
pub struct CardShowArgs {
    /// Deck file to read.
    pub deck: PathBuf,
    /// 1-based index.
    #[arg(short = 'i', long = "index")]
    pub index: usize,
    /// Render punched rows using ASCII art.
    #[arg(long)]
    pub interpret: bool,
}

/// Arguments for `punch card patch`.
#[derive(Args, Debug)]
pub struct CardPatchArgs {
    /// Deck file to modify.
    pub deck: PathBuf,
    /// Text for the corrective card.
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long = "from")]
    pub from: Option<PathBuf>,
    #[arg(long)]
    pub note: Option<String>,
}

/// Execute a card command.
pub fn handle(command: CardCommand) -> Result<()> {
    match command {
        CardCommand::Add(args) => add(args),
        CardCommand::Type(args) => type_cards(args),
        CardCommand::Replace(args) => replace(args),
        CardCommand::Show(args) => show(args),
        CardCommand::Patch(args) => patch(args),
    }
}

fn add(args: CardAddArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
    let template = match &args.template {
        Some(name) => Some(
            TemplateRegistry::get(name)
                .with_context(|| format!("template '{}' not found", name))?,
        ),
        None => None,
    };
    let text = read_text_arg(args.text.clone(), args.from.clone())?;
    let lines = split_lines_fixed(&text);
    let chosen_type: CardType = args.card_type.into();
    for (i, line) in lines.iter().enumerate() {
        let mut record = if let Some(tpl) = template {
            tpl.apply(line)?
        } else {
            CardRecord::from_text(line, EncodingKind::Hollerith, chosen_type.clone())?
        };
        record.meta = CardMeta {
            note: args.note.clone(),
            color: args.color.clone(),
        };
        if let Some(pos) = args.position {
            let idx = pos.saturating_sub(1) + i;
            deck.insert_card(idx, record)?;
        } else {
            deck.append_card(record)?;
        }
    }
    deck.log_action("card add");
    deck.save(&args.deck)?;
    println!("Added {} card(s) into {}", lines.len(), args.deck.display());
    Ok(())
}

fn type_cards(args: CardTypeArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
    let template = match &args.template {
        Some(name) => Some(
            TemplateRegistry::get(name)
                .with_context(|| format!("template '{}' not found", name))?,
        ),
        None => None,
    };
    let buffer = read_stdin()?;
    let lines = split_lines_fixed(&buffer);
    let chosen_type: CardType = args.card_type.into();
    for line in lines {
        let mut record = if let Some(tpl) = template {
            tpl.apply(&line)?
        } else {
            CardRecord::from_text(&line, EncodingKind::Hollerith, chosen_type.clone())?
        };
        record.meta = CardMeta {
            note: args.note.clone(),
            color: args.color.clone(),
        };
        deck.append_card(record)?;
    }
    deck.log_action("card type");
    deck.save(&args.deck)?;
    println!("Typed cards appended to {}", args.deck.display());
    Ok(())
}

fn replace(args: CardReplaceArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
    if args.index == 0 || args.index > deck.cards.len() {
        return Err(anyhow!(
            "card index {} out of range 1..{}",
            args.index,
            deck.cards.len()
        ));
    }
    let text = read_text_arg(args.text.clone(), args.from.clone())?;
    let existing_type = deck.cards[args.index - 1].card_type.clone();
    let mut record = CardRecord::from_text(&text, EncodingKind::Hollerith, existing_type)?;
    if let Some(kind) = args.card_type {
        record.card_type = kind.into();
    }
    record.meta = CardMeta {
        note: args.note.clone(),
        color: args.color.clone(),
    };
    deck.replace_card(args.index - 1, record)?;
    deck.log_action(format!("card replace {}", args.index));
    deck.save(&args.deck)?;
    println!("Replaced card {} in {}", args.index, args.deck.display());
    Ok(())
}

fn show(args: CardShowArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    if args.index == 0 || args.index > deck.cards.len() {
        return Err(anyhow!(
            "card index {} out of range 1..{}",
            args.index,
            deck.cards.len()
        ));
    }
    let card = &deck.cards[args.index - 1];
    println!("Card {} of {}", args.index, deck.cards.len());
    println!("Type: {:?}", card.card_type);
    if let Some(seq) = card.seq {
        println!("Sequence: {}", seq);
    }
    if let Some(meta) = card.meta.note.as_ref() {
        println!("Note: {}", meta);
    }
    if let Some(color) = card.meta.color.as_ref() {
        println!("Color: {}", color);
    }
    match card.text.as_ref() {
        Some(text) => {
            println!("Text:\n{}", text);
        }
        None => println!("(card stored as punches)"),
    }
    if args.interpret {
        let encoder = Ibm029Encoder::new();
        let punch = card.to_punch_card(&encoder)?;
        println!("{}", punch.render(RenderStyle::AsciiX));
    }
    Ok(())
}

fn patch(args: CardPatchArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
    let text = read_text_arg(args.text.clone(), args.from.clone())?;
    let mut record = CardRecord::from_text(&text, EncodingKind::Hollerith, CardType::Patch)?;
    record.meta = CardMeta {
        note: args.note.clone().or_else(|| Some("patch card".to_string())),
        color: Some("amber".to_string()),
    };
    deck.append_card(record)?;
    deck.log_action("card patch");
    deck.save(&args.deck)?;
    println!("Appended patch card to {}", args.deck.display());
    Ok(())
}
