//! Audit and hashing commands (`punch audit ...`).

use std::path::PathBuf;

use crate::cli::utils::load_deck;
use anyhow::Result;
use clap::{Args, Subcommand};

/// Audit subcommands.
#[derive(Subcommand, Debug)]
pub enum AuditCommand {
    /// Compute SHA-256 hash over deck content.
    Hash(AuditHashArgs),
    /// Show audited history events.
    Log(AuditLogArgs),
}

/// Arguments for `punch audit hash`.
#[derive(Args, Debug)]
pub struct AuditHashArgs {
    /// Deck file to hash.
    pub deck: PathBuf,
}

/// Arguments for `punch audit log`.
#[derive(Args, Debug)]
pub struct AuditLogArgs {
    /// Deck file to inspect.
    pub deck: PathBuf,
}

/// Execute an audit command.
pub fn handle(command: AuditCommand) -> Result<()> {
    match command {
        AuditCommand::Hash(args) => hash(args),
        AuditCommand::Log(args) => log(args),
    }
}

fn hash(args: AuditHashArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    let digest = deck.hash()?;
    println!("{}", digest);
    Ok(())
}

fn log(args: AuditLogArgs) -> Result<()> {
    let deck = load_deck(args.deck.as_path())?;
    if deck.header.history.is_empty() {
        println!("No audit events recorded.");
    } else {
        for event in &deck.header.history {
            println!("{} {} - {}", event.timestamp, event.actor, event.action);
        }
    }
    Ok(())
}
