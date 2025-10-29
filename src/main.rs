use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use punchcard::{
    CardMeta, CardRecord, CardType, ColumnRange, Deck, DeckHeader, EncodingKind, Ibm029Encoder,
    RenderStyle, TemplateRegistry, encode_text_to_deck,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Deck { command } => handle_deck(command),
        Command::Card { command } => handle_card(command),
        Command::Seq { command } => handle_seq(command),
        Command::Render { command } => handle_render(command),
        Command::Template { command } => handle_template(command),
        Command::Encode { command } => handle_encode(command),
        Command::Audit { command } => handle_audit(command),
        Command::Verify { command } => handle_verify(command),
    }
}

#[derive(Parser, Debug)]
#[command(name = "punch", version, about = "IBM punch card workflow toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Deck {
        #[command(subcommand)]
        command: DeckCommand,
    },
    Card {
        #[command(subcommand)]
        command: CardCommand,
    },
    Seq {
        #[command(subcommand)]
        command: SeqCommand,
    },
    Render {
        #[command(subcommand)]
        command: RenderCommand,
    },
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    Encode {
        #[command(subcommand)]
        command: EncodeCommand,
    },
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    Verify {
        #[command(subcommand)]
        command: VerifyCommand,
    },
}

#[derive(Subcommand, Debug)]
enum DeckCommand {
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

#[derive(Args, Debug)]
struct DeckInitArgs {
    /// Output deck path (JSONL).
    path: PathBuf,
    /// Logical language template (fortran/cobol/jcl/assembler)
    #[arg(short = 'l', long)]
    language: Option<String>,
    /// Column template shortcut.
    #[arg(short = 't', long)]
    template: Option<String>,
    /// Protected column ranges, e.g. --protect 73-80
    #[arg(long = "protect", value_parser = parse_column_range)]
    protect: Vec<ColumnRange>,
}

#[derive(Args, Debug)]
struct DeckImportArgs {
    /// 80-column text file to import.
    source: PathBuf,
    /// Output deck file.
    #[arg(short = 'o', long)]
    out: PathBuf,
    /// Encoding to mark on imported cards.
    #[arg(long, default_value_t = EncodingArg::Hollerith, value_enum)]
    encoding: EncodingArg,
    /// Card type for imported lines.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    card_type: CardTypeArg,
}

#[derive(Args, Debug)]
struct DeckExportArgs {
    /// Source deck file.
    deck: PathBuf,
    /// Output file path (`-` for stdout).
    #[arg(short = 'o', long)]
    out: PathBuf,
    /// Export format (text80, deck)
    #[arg(long, default_value_t = DeckExportFormat::Text80, value_enum)]
    format: DeckExportFormat,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum DeckExportFormat {
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

#[derive(Args, Debug)]
struct DeckInfoArgs {
    /// Deck file to inspect.
    deck: PathBuf,
}

#[derive(Args, Debug)]
struct DeckMergeArgs {
    /// Input deck files to merge.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Output deck file.
    #[arg(short = 'o', long)]
    out: PathBuf,
}

#[derive(Args, Debug)]
struct DeckSliceArgs {
    /// Source deck file.
    deck: PathBuf,
    /// Range expression, e.g. 1..10,25,30..$
    #[arg(long)]
    range: String,
    /// Output deck file.
    #[arg(short = 'o', long)]
    out: PathBuf,
}

#[derive(Subcommand, Debug)]
enum CardCommand {
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

#[derive(Args, Debug)]
struct CardAddArgs {
    deck: PathBuf,
    /// Direct text for the card (80 chars or fewer).
    #[arg(long)]
    text: Option<String>,
    /// Load card text from file (`-` for stdin).
    #[arg(long = "from")]
    from: Option<PathBuf>,
    /// Apply template defaults.
    #[arg(long)]
    template: Option<String>,
    /// Explicit card type.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    card_type: CardTypeArg,
    /// Optional human note.
    #[arg(long)]
    note: Option<String>,
    /// Optional color hint.
    #[arg(long)]
    color: Option<String>,
    /// Insert at 1-based position (defaults to append).
    #[arg(long)]
    position: Option<usize>,
}

#[derive(Args, Debug)]
struct CardTypeArgs {
    deck: PathBuf,
    /// Apply template defaults during typing.
    #[arg(long)]
    template: Option<String>,
    /// Explicit card type.
    #[arg(long = "type", default_value_t = CardTypeArg::Code, value_enum)]
    card_type: CardTypeArg,
    /// Optional human note applied to all typed cards.
    #[arg(long)]
    note: Option<String>,
    /// Optional color hint.
    #[arg(long)]
    color: Option<String>,
}

#[derive(Args, Debug)]
struct CardReplaceArgs {
    deck: PathBuf,
    /// 1-based index of the card to replace.
    index: usize,
    #[arg(long)]
    text: Option<String>,
    #[arg(long = "from")]
    from: Option<PathBuf>,
    #[arg(long)]
    note: Option<String>,
    #[arg(long)]
    color: Option<String>,
    #[arg(long = "type")]
    #[arg(value_enum)]
    card_type: Option<CardTypeArg>,
}

#[derive(Args, Debug)]
struct CardShowArgs {
    deck: PathBuf,
    /// 1-based index.
    index: usize,
    /// Render punched rows using ASCII art.
    #[arg(long)]
    interpret: bool,
}

#[derive(Args, Debug)]
struct CardPatchArgs {
    deck: PathBuf,
    /// Text for the corrective card.
    #[arg(long)]
    text: Option<String>,
    #[arg(long = "from")]
    from: Option<PathBuf>,
    #[arg(long)]
    note: Option<String>,
}

#[derive(Subcommand, Debug)]
enum SeqCommand {
    /// Apply sequential numbers to cards.
    Number(SeqNumberArgs),
    /// Sort cards by existing sequence numbers.
    Sort(SeqSortArgs),
}

#[derive(Args, Debug)]
struct SeqNumberArgs {
    deck: PathBuf,
    #[arg(long, default_value_t = 10)]
    start: usize,
    #[arg(long, default_value_t = 10)]
    step: usize,
}

#[derive(Args, Debug)]
struct SeqSortArgs {
    deck: PathBuf,
}

#[derive(Subcommand, Debug)]
enum RenderCommand {
    /// Produce interpreter-style listing.
    Interpret(RenderInterpretArgs),
    /// Emit a card-by-card textual listing.
    Listing(RenderListingArgs),
}

#[derive(Args, Debug)]
struct RenderInterpretArgs {
    deck: PathBuf,
    /// Output file (`-` for stdout)
    #[arg(short = 'o', long)]
    out: Option<PathBuf>,
    /// Rendering style.
    #[arg(long, default_value_t = RenderStyleArg::AsciiX, value_enum)]
    style: RenderStyleArg,
}

#[derive(Args, Debug)]
struct RenderListingArgs {
    deck: PathBuf,
    /// Output file (`-` for stdout)
    #[arg(short = 'o', long)]
    out: Option<PathBuf>,
    /// Rendering style for punch visualization.
    #[arg(long, default_value_t = RenderStyleArg::AsciiX, value_enum)]
    style: RenderStyleArg,
}

#[derive(Subcommand, Debug)]
enum TemplateCommand {
    List,
    Show(TemplateShowArgs),
}

#[derive(Args, Debug)]
struct TemplateShowArgs {
    name: String,
}

#[derive(Subcommand, Debug)]
enum EncodeCommand {
    /// Encode text into punch card deck.
    Text(EncodeTextArgs),
}

#[derive(Args, Debug)]
struct EncodeTextArgs {
    /// Input text (falls back to stdin if omitted).
    #[arg(long)]
    text: Option<String>,
    /// Read input from file (`-` for stdin).
    #[arg(long = "from")]
    from: Option<PathBuf>,
    /// Render ASCII representation.
    #[arg(long)]
    render: bool,
}

#[derive(Subcommand, Debug)]
enum AuditCommand {
    /// Compute SHA-256 hash over deck content.
    Hash(AuditHashArgs),
    /// Show audited history events.
    Log(AuditLogArgs),
}

#[derive(Args, Debug)]
struct AuditHashArgs {
    deck: PathBuf,
}

#[derive(Args, Debug)]
struct AuditLogArgs {
    deck: PathBuf,
}

#[derive(Subcommand, Debug)]
enum VerifyCommand {
    /// Capture the current deck snapshot for verification.
    Start(VerifyStartArgs),
    /// Compare a second pass against recorded snapshot.
    Pass(VerifyPassArgs),
    /// Display the latest verification diff.
    Report(VerifyReportArgs),
}

#[derive(Args, Debug)]
struct VerifyStartArgs {
    deck: PathBuf,
}

#[derive(Args, Debug)]
struct VerifyPassArgs {
    deck: PathBuf,
    /// Text file to compare (`-` for stdin)
    #[arg(long = "from")]
    from: Option<PathBuf>,
    /// Treat any difference as an error.
    #[arg(long)]
    strict: bool,
    /// Ignore specified column ranges during comparison.
    #[arg(long = "mask", value_parser = parse_column_range)]
    mask: Vec<ColumnRange>,
}

#[derive(Args, Debug)]
struct VerifyReportArgs {
    deck: PathBuf,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum EncodingArg {
    Hollerith,
    Ascii,
    Ebcdic,
}

impl From<EncodingArg> for EncodingKind {
    fn from(value: EncodingArg) -> EncodingKind {
        match value {
            EncodingArg::Hollerith => EncodingKind::Hollerith,
            EncodingArg::Ascii => EncodingKind::Ascii,
            EncodingArg::Ebcdic => EncodingKind::Ebcdic,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum CardTypeArg {
    Code,
    Data,
    Jcl,
    Comment,
    Separator,
    Patch,
}

impl From<CardTypeArg> for CardType {
    fn from(value: CardTypeArg) -> CardType {
        match value {
            CardTypeArg::Code => CardType::Code,
            CardTypeArg::Data => CardType::Data,
            CardTypeArg::Jcl => CardType::Jcl,
            CardTypeArg::Comment => CardType::Comment,
            CardTypeArg::Separator => CardType::Separator,
            CardTypeArg::Patch => CardType::Patch,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
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

fn handle_deck(command: DeckCommand) -> Result<()> {
    match command {
        DeckCommand::Init(args) => {
            if let Some(tpl) = &args.template {
                TemplateRegistry::get(tpl)
                    .with_context(|| format!("template '{}' not found", tpl))?;
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
        }
        DeckCommand::Import(args) => {
            let contents = fs::read_to_string(&args.source)
                .with_context(|| format!("failed to read {}", args.source.display()))?;
            let mut deck = Deck::new(DeckHeader::new(None, None, Vec::new()));
            let encoding: EncodingKind = args.encoding.into();
            let card_type: CardType = args.card_type.into();
            for (idx, line) in contents.lines().enumerate() {
                let record = CardRecord::from_text(line, encoding, card_type.clone())
                    .with_context(|| {
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
            deck.save(&args.out)?;
            println!(
                "Imported {} cards into {}",
                deck.cards.len(),
                args.out.display()
            );
        }
        DeckCommand::Export(args) => {
            let deck = load_deck(&args.deck)?;
            match args.format {
                DeckExportFormat::Text80 => {
                    let text = deck.as_text().join("\n");
                    write_output(&args.out, &text)?;
                }
                DeckExportFormat::Deck => {
                    let mut clone = deck.clone();
                    clone.save(&args.out)?;
                }
            }
            println!(
                "Exported deck {} as {:?} -> {}",
                args.deck.display(),
                args.format,
                args.out.display()
            );
        }
        DeckCommand::Info(args) => {
            let deck = load_deck(&args.deck)?;
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
        }
        DeckCommand::Merge(args) => {
            if args.inputs.len() < 2 {
                return Err(anyhow!("merge requires at least two input decks"));
            }
            let mut merged: Option<Deck> = None;
            for input in &args.inputs {
                let deck = load_deck(input)?;
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
                args.out.display()
            ));
            result.save(&args.out)?;
            println!(
                "Merged {} cards into {}",
                result.cards.len(),
                args.out.display()
            );
        }
        DeckCommand::Slice(args) => {
            let source = load_deck(&args.deck)?;
            let indexes = parse_range_expression(&args.range, source.cards.len())?;
            let mut sliced = source.slice_indices(&indexes)?;
            sliced.log_action(format!("slice {} -> {}", args.range, args.out.display()));
            sliced.save(&args.out)?;
            println!(
                "Sliced {} cards into {}",
                sliced.cards.len(),
                args.out.display()
            );
        }
    }
    Ok(())
}

fn handle_card(command: CardCommand) -> Result<()> {
    match command {
        CardCommand::Add(args) => {
            let mut deck = load_deck(&args.deck)?;
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
        }
        CardCommand::Type(args) => {
            let mut deck = load_deck(&args.deck)?;
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
        }
        CardCommand::Replace(args) => {
            let mut deck = load_deck(&args.deck)?;
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
        }
        CardCommand::Show(args) => {
            let deck = load_deck(&args.deck)?;
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
        }
        CardCommand::Patch(args) => {
            let mut deck = load_deck(&args.deck)?;
            let text = read_text_arg(args.text.clone(), args.from.clone())?;
            let mut record =
                CardRecord::from_text(&text, EncodingKind::Hollerith, CardType::Patch)?;
            record.meta = CardMeta {
                note: args.note.clone().or_else(|| Some("patch card".to_string())),
                color: Some("amber".to_string()),
            };
            deck.append_card(record)?;
            deck.log_action("card patch");
            deck.save(&args.deck)?;
            println!("Appended patch card to {}", args.deck.display());
        }
    }
    Ok(())
}

fn handle_seq(command: SeqCommand) -> Result<()> {
    match command {
        SeqCommand::Number(args) => {
            let mut deck = load_deck(&args.deck)?;
            deck.number_sequence(args.start, args.step);
            deck.log_action(format!(
                "seq number start={} step={}",
                args.start, args.step
            ));
            deck.save(&args.deck)?;
            println!(
                "Applied sequence numbers (start {}, step {}) to {}",
                args.start,
                args.step,
                args.deck.display()
            );
        }
        SeqCommand::Sort(args) => {
            let mut deck = load_deck(&args.deck)?;
            deck.sort_by_sequence();
            deck.log_action("seq sort");
            deck.save(&args.deck)?;
            println!("Sorted {} by sequence numbers", args.deck.display());
        }
    }
    Ok(())
}

fn handle_render(command: RenderCommand) -> Result<()> {
    match command {
        RenderCommand::Interpret(args) => {
            let deck = load_deck(&args.deck)?;
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
            if let Some(path) = args.out {
                write_output(&path, &output)?;
                println!(
                    "Wrote interpreted listing for {} to {}",
                    args.deck.display(),
                    path.display()
                );
            } else {
                print!("{}", output);
            }
        }
        RenderCommand::Listing(args) => {
            let deck = load_deck(&args.deck)?;
            let encoder = Ibm029Encoder::new();
            let punch_deck = deck
                .to_punch_deck(&encoder)
                .context("failed to render deck with IBM029 encoder")?;
            let mut output = String::new();
            for (idx, (record, card)) in deck.cards.iter().zip(punch_deck.cards.iter()).enumerate()
            {
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
            if let Some(path) = args.out {
                write_output(&path, &output)?;
                println!(
                    "Wrote listing for {} to {}",
                    args.deck.display(),
                    path.display()
                );
            } else {
                print!("{}", output);
            }
        }
    }
    Ok(())
}

fn handle_template(command: TemplateCommand) -> Result<()> {
    match command {
        TemplateCommand::List => {
            println!("Available templates:");
            for tpl in TemplateRegistry::list() {
                println!("  - {}: {}", tpl.name, tpl.description);
            }
        }
        TemplateCommand::Show(args) => {
            let tpl = TemplateRegistry::get(&args.name)
                .with_context(|| format!("template '{}' not found", args.name))?;
            println!("Template: {}", tpl.name);
            println!("{}", tpl.description);
            for column in tpl.columns {
                println!(
                    "  {:>2}-{:>2}: {}",
                    column.range.start, column.range.end, column.label
                );
            }
        }
    }
    Ok(())
}

fn handle_encode(command: EncodeCommand) -> Result<()> {
    match command {
        EncodeCommand::Text(args) => {
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
        }
    }
    Ok(())
}

fn handle_audit(command: AuditCommand) -> Result<()> {
    match command {
        AuditCommand::Hash(args) => {
            let deck = load_deck(&args.deck)?;
            let digest = deck.hash()?;
            println!("{}", digest);
        }
        AuditCommand::Log(args) => {
            let deck = load_deck(&args.deck)?;
            if deck.header.history.is_empty() {
                println!("No audit events recorded.");
            } else {
                for event in &deck.header.history {
                    println!("{} {} - {}", event.timestamp, event.actor, event.action);
                }
            }
        }
    }
    Ok(())
}

fn handle_verify(command: VerifyCommand) -> Result<()> {
    match command {
        VerifyCommand::Start(args) => {
            let deck = load_deck(&args.deck)?;
            let snapshot_path = verify_snapshot_path(&args.deck);
            let text = deck.as_text().join("\n");
            write_output(&snapshot_path, &text)?;
            println!(
                "Stored verification baseline at {}",
                snapshot_path.display()
            );
        }
        VerifyCommand::Pass(args) => {
            load_deck(&args.deck)?;
            let snapshot_path = verify_snapshot_path(&args.deck);
            if !snapshot_path.exists() {
                return Err(anyhow!(
                    "no verification snapshot found at {}. Run `punch verify start` first.",
                    snapshot_path.display()
                ));
            }
            let expected = fs::read_to_string(&snapshot_path)
                .with_context(|| format!("failed to read {}", snapshot_path.display()))?;
            let actual = read_text_arg(None, args.from.clone())?;
            let (diff, changed) = diff_text(&expected, &actual, &args.mask);
            let diff_path = verify_diff_path(&args.deck);
            write_output(&diff_path, &diff)?;
            if args.strict && changed {
                return Err(anyhow!(
                    "verification failed; see diff at {}",
                    diff_path.display()
                ));
            }
            if changed {
                println!("Verification diff written to {}", diff_path.display());
            } else {
                println!(
                    "Verification passed with ignored masks; diff stored at {}",
                    diff_path.display()
                );
            }
        }
        VerifyCommand::Report(args) => {
            let diff_path = verify_diff_path(&args.deck);
            if !diff_path.exists() {
                println!(
                    "No verification diff at {}. Run `punch verify pass` first.",
                    diff_path.display()
                );
                return Ok(());
            }
            let diff = fs::read_to_string(&diff_path)
                .with_context(|| format!("failed to read {}", diff_path.display()))?;
            println!("{}", diff);
        }
    }
    Ok(())
}

fn load_deck(path: &Path) -> Result<Deck> {
    Deck::load(path).with_context(|| format!("failed to read deck {}", path.display()))
}

fn read_text_arg(text: Option<String>, from: Option<PathBuf>) -> Result<String> {
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

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("failed to read from stdin")?;
    Ok(buffer)
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if path.as_os_str() == "-" {
        io::stdout().write_all(content.as_bytes())?;
        return Ok(());
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn parse_column_range(input: &str) -> Result<ColumnRange, String> {
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

fn parse_range_expression(expr: &str, deck_len: usize) -> Result<Vec<usize>> {
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

fn split_lines_fixed(input: &str) -> Vec<String> {
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

fn diff_text(expected: &str, actual: &str, mask: &[ColumnRange]) -> (String, bool) {
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

fn verify_snapshot_path(deck: &Path) -> PathBuf {
    let mut path = deck.to_path_buf();
    path.set_extension("verify.base");
    path
}

fn verify_diff_path(deck: &Path) -> PathBuf {
    let mut path = deck.to_path_buf();
    path.set_extension("verify.diff");
    path
}
