//! Verification workflow (`punch verify ...`).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand};
use punchcard::ColumnRange;

use crate::cli::utils::{
    diff_text, load_deck, parse_column_range, read_text_arg, verify_diff_path,
    verify_snapshot_path, write_output,
};

/// Verification subcommands.
#[derive(Subcommand, Debug)]
pub enum VerifyCommand {
    /// Capture the current deck snapshot for verification.
    Start(VerifyStartArgs),
    /// Compare a second pass against recorded snapshot.
    Pass(VerifyPassArgs),
    /// Display the latest verification diff.
    Report(VerifyReportArgs),
}

/// Arguments for `punch verify start`.
#[derive(Args, Debug)]
pub struct VerifyStartArgs {
    /// Deck file to snapshot.
    pub deck: PathBuf,
}

/// Arguments for `punch verify pass`.
#[derive(Args, Debug)]
pub struct VerifyPassArgs {
    /// Deck file being verified.
    pub deck: PathBuf,
    /// Text file to compare (`-` for stdin).
    #[arg(long = "from")]
    pub from: Option<PathBuf>,
    /// Treat any difference as an error.
    #[arg(long)]
    pub strict: bool,
    /// Ignore specified column ranges during comparison.
    #[arg(long = "mask", value_parser = parse_column_range)]
    pub mask: Vec<ColumnRange>,
}

/// Arguments for `punch verify report`.
#[derive(Args, Debug)]
pub struct VerifyReportArgs {
    /// Deck file to inspect.
    pub deck: PathBuf,
}

/// Execute a verification command.
pub fn handle(command: VerifyCommand) -> Result<()> {
    match command {
        VerifyCommand::Start(args) => start(args),
        VerifyCommand::Pass(args) => pass(args),
        VerifyCommand::Report(args) => report(args),
    }
}

fn start(args: VerifyStartArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    let snapshot_path = verify_snapshot_path(&args.deck);
    let text = deck.as_text().join("\n");
    write_output(&snapshot_path, &text)?;
    println!(
        "Stored verification baseline at {}",
        snapshot_path.display()
    );
    Ok(())
}

fn pass(args: VerifyPassArgs) -> Result<()> {
    load_deck(args.deck.as_path())?;
    let snapshot_path = verify_snapshot_path(&args.deck);
    if !snapshot_path.exists() {
        return Err(anyhow!(
            "no verification snapshot found at {}. Run `punch verify start` first.",
            snapshot_path.display()
        ));
    }
    let expected = std::fs::read_to_string(&snapshot_path)
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
    Ok(())
}

fn report(args: VerifyReportArgs) -> Result<()> {
    let diff_path = verify_diff_path(&args.deck);
    if !diff_path.exists() {
        println!(
            "No verification diff at {}. Run `punch verify pass` first.",
            diff_path.display()
        );
        return Ok(());
    }
    let diff = std::fs::read_to_string(&diff_path)
        .with_context(|| format!("failed to read {}", diff_path.display()))?;
    println!("{}", diff);
    Ok(())
}
