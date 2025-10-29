//! Deck lifecycle commands (`punch deck ...`).

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand, ValueEnum};
use punchcard::{
    CardRecord, CardType, ColumnRange, Deck, DeckHeader, EncodingKind, TemplateRegistry,
};

use crate::cli::common::{CardTypeArg, EncodingArg};
use crate::cli::utils::{load_deck, parse_column_range, parse_range_expression, write_output};

/// Supported `punch deck` subcommands.
#[derive(Subcommand, Debug)]
pub enum DeckCommand {
    /// Initialize a new deck file with optional template metadata.
    Init(DeckInitArgs),
    /// Import 80-column text into a deck file.
    Import(DeckImportArgs),
    /// Export an existing deck into another format.
    Export(DeckExportArgs),
    /// Show deck metadata summary.
    Info(DeckInfoArgs),
    /// Merge multiple deck files into a new deck.
    Merge(DeckMergeArgs),
    /// Slice a deck by card indices or ranges.
    Slice(DeckSliceArgs),
}

/// Arguments for `punch deck init`.
#[derive(Args, Debug)]
pub struct DeckInitArgs {
    /// Output deck path (JSONL).
    pub path: PathBuf,
    /// Logical language template (fortran/cobol/jcl/assembler)
    #[arg(short = 'l', long)]
    pub language: Option<String>,
    /// Column template shortcut.
    #[arg(short = 't', long)]
    pub template: Option<String>,
    /// Protected column ranges, e.g. --protect 73-80
    #[arg(long = "protect", value_parser = parse_column_range)]
    pub protect: Vec<ColumnRange>,
}

/// Arguments for `punch deck import`.
#[derive(Args, Debug)]
pub struct DeckImportArgs {
    /// 80-column text file to import.
    pub source: PathBuf,
    /// Output deck file.
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    /// Encoding to mark on imported cards.
    #[arg(long, default_value_t = EncodingArg::Hollerith, value_enum)]
    pub encoding: EncodingArg,
    /// Card type for imported lines.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    pub card_type: CardTypeArg,
}

/// Arguments for `punch deck export`.
#[derive(Args, Debug)]
pub struct DeckExportArgs {
    /// Source deck file.
    pub deck: PathBuf,
    /// Output file path (`-` for stdout).
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    /// Export format (text80, deck)
    #[arg(long, default_value_t = DeckExportFormat::Text80, value_enum)]
    pub format: DeckExportFormat,
}

/// Export format for deck content.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum DeckExportFormat {
    Text80,
    Deck,
}

impl fmt::Display for DeckExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeckExportFormat::Text80 => write!(f, "text80"),
            DeckExportFormat::Deck => write!(f, "deck"),
        }
    }
}

/// Arguments for `punch deck info`.
#[derive(Args, Debug)]
pub struct DeckInfoArgs {
    /// Deck file to inspect.
    pub deck: PathBuf,
}

/// Arguments for `punch deck merge`.
#[derive(Args, Debug)]
pub struct DeckMergeArgs {
    /// Input deck files to merge.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    /// Output deck file.
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

/// Arguments for `punch deck slice`.
#[derive(Args, Debug)]
pub struct DeckSliceArgs {
    /// Source deck file.
    pub deck: PathBuf,
    /// Range expression, e.g. 1..10,25,30..$
    #[arg(short = 'r', long = "range")]
    pub range: String,
    /// Output deck file.
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

/// Execute a deck command.
pub fn handle(command: DeckCommand) -> Result<()> {
    match command {
        DeckCommand::Init(args) => init(args),
        DeckCommand::Import(args) => import(args),
        DeckCommand::Export(args) => export(args),
        DeckCommand::Info(args) => info(args),
        DeckCommand::Merge(args) => merge(args),
        DeckCommand::Slice(args) => slice(args),
    }
}

fn init(args: DeckInitArgs) -> Result<()> {
    if let Some(tpl) = &args.template {
        TemplateRegistry::get(tpl).with_context(|| format!("template '{}' not found", tpl))?;
    }
    let header = DeckHeader::new(
        args.language.clone(),
        args.template.clone(),
        args.protect.clone(),
    );
    let mut deck = Deck::new(header);
    deck.log_action("deck init");
    deck.save(&args.path)?;
    println!(
        "Created deck {} (language: {:?}, template: {:?})",
        args.path.display(),
        args.language,
        args.template
    );
    Ok(())
}

fn import(args: DeckImportArgs) -> Result<()> {
    let contents = std::fs::read_to_string(&args.source)
        .with_context(|| format!("failed to read {}", args.source.display()))?;
    let mut deck = Deck::new(DeckHeader::new(None, None, Vec::new()));
    let encoding: EncodingKind = args.encoding.into();
    let card_type: CardType = args.card_type.into();
    for (idx, line) in contents.lines().enumerate() {
        let record =
            CardRecord::from_text(line, encoding, card_type.clone()).with_context(|| {
                format!(
                    "line {} in {} exceeds 80 columns",
                    idx + 1,
                    args.source.display()
                )
            })?;
        deck.append_card(record)?;
    }
    deck.log_action(format!(
        "import from {} as {:?}",
        args.source.display(),
        encoding
    ));
    deck.save(&args.output)?;
    println!(
        "Imported {} cards into {}",
        deck.cards.len(),
        args.output.display()
    );
    Ok(())
}

fn export(args: DeckExportArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    match args.format {
        DeckExportFormat::Text80 => {
            let text = deck.as_text().join("\n");
            write_output(&args.output, &text)?;
        }
        DeckExportFormat::Deck => {
            let mut clone = deck.clone();
            clone.save(&args.output)?;
        }
    }
    println!(
        "Exported deck {} as {:?} -> {}",
        args.deck.display(),
        args.format,
        args.output.display()
    );
    Ok(())
}

fn info(args: DeckInfoArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    println!("Deck: {}", args.deck.display());
    println!("Cards: {}", deck.cards.len());
    println!(
        "Language: {}",
        deck.header
            .language
            .as_deref()
            .unwrap_or_else(|| "(unspecified)")
    );
    if let Some(template) = &deck.header.template {
        println!("Template: {}", template);
    }
    if !deck.header.protected_cols.is_empty() {
        let ranges: Vec<String> = deck
            .header
            .protected_cols
            .iter()
            .map(|r| format!("{}-{}", r.start, r.end))
            .collect();
        println!("Protected cols: {}", ranges.join(", "));
    }
    println!("History entries: {}", deck.header.history.len());
    Ok(())
}

fn merge(args: DeckMergeArgs) -> Result<()> {
    if args.inputs.len() < 2 {
        return Err(anyhow!("merge requires at least two input decks"));
    }
    let mut merged: Option<Deck> = None;
    for input in &args.inputs {
        let deck = load_deck(input.as_path())?;
        merged = Some(match merged {
            None => deck,
            Some(mut acc) => {
                acc.merge_from(&deck)?;
                acc
            }
        });
    }
    let mut result = merged.expect("at least one deck");
    result.log_action(format!(
        "merge {} decks into {}",
        args.inputs.len(),
        args.output.display()
    ));
    result.save(&args.output)?;
    println!(
        "Merged {} cards into {}",
        result.cards.len(),
        args.output.display()
    );
    Ok(())
}

fn slice(args: DeckSliceArgs) -> Result<()> {
    let source = load_deck(args.deck.as_path())?;
    let indexes = parse_range_expression(&args.range, source.cards.len())?;
    let mut sliced = source.slice_indices(&indexes)?;
    sliced.log_action(format!("slice {} -> {}", args.range, args.output.display()));
    sliced.save(&args.output)?;
    println!(
        "Sliced {} cards into {}",
        sliced.cards.len(),
        args.output.display()
    );
    Ok(())
}
