//! Sequence number operations (`punch seq ...`).

use std::path::PathBuf;

use crate::cli::utils::load_deck;
use anyhow::Result;
use clap::{Args, Subcommand};

/// Sequence-related subcommands.
#[derive(Subcommand, Debug)]
pub enum SeqCommand {
    /// Apply sequential numbers to cards.
    Number(SeqNumberArgs),
    /// Sort cards by existing sequence numbers.
    Sort(SeqSortArgs),
}

/// Arguments for numbering a deck.
#[derive(Args, Debug)]
pub struct SeqNumberArgs {
    /// Deck file to update.
    pub deck: PathBuf,
    /// Starting sequence value.
    #[arg(long, default_value_t = 10)]
    pub start: usize,
    /// Step applied between cards.
    #[arg(long, default_value_t = 10)]
    pub step: usize,
}

/// Arguments for sorting cards by sequence number.
#[derive(Args, Debug)]
pub struct SeqSortArgs {
    /// Deck file to update.
    pub deck: PathBuf,
}

/// Execute a sequence command.
pub fn handle(command: SeqCommand) -> Result<()> {
    match command {
        SeqCommand::Number(args) => number(args),
        SeqCommand::Sort(args) => sort(args),
    }
}

fn number(args: SeqNumberArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
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
    Ok(())
}

fn sort(args: SeqSortArgs) -> Result<()> {
    let mut deck = load_deck(args.deck.as_path())?;
    deck.sort_by_sequence();
    deck.log_action("seq sort");
    deck.save(&args.deck)?;
    println!("Sorted {} by sequence numbers", args.deck.display());
    Ok(())
}
