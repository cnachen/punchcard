//! Command-line interface wiring for the `punch` binary.
//!
//! This module owns the clap definitions and delegates execution to
//! specialized submodules that encapsulate each command family.

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod audit;
pub mod card;
pub mod common;
pub mod deck;
pub mod encode;
pub mod render;
pub mod seq;
pub mod template;
pub mod utils;
pub mod verify;

/// Parsed CLI entrypoint for the `punch` binary.
#[derive(Parser, Debug)]
#[command(name = "punch", version, about = "IBM punch card workflow toolkit")]
pub struct Cli {
    /// Top-level command to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// High-level command families made available to end users.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(subcommand)]
    Deck(deck::DeckCommand),
    #[command(subcommand)]
    Card(card::CardCommand),
    #[command(subcommand)]
    Seq(seq::SeqCommand),
    #[command(subcommand)]
    Render(render::RenderCommand),
    #[command(subcommand)]
    Template(template::TemplateCommand),
    #[command(subcommand)]
    Encode(encode::EncodeCommand),
    #[command(subcommand)]
    Audit(audit::AuditCommand),
    #[command(subcommand)]
    Verify(verify::VerifyCommand),
}

/// Execute the requested command.
pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Deck(cmd) => deck::handle(cmd),
        Command::Card(cmd) => card::handle(cmd),
        Command::Seq(cmd) => seq::handle(cmd),
        Command::Render(cmd) => render::handle(cmd),
        Command::Template(cmd) => template::handle(cmd),
        Command::Encode(cmd) => encode::handle(cmd),
        Command::Audit(cmd) => audit::handle(cmd),
        Command::Verify(cmd) => verify::handle(cmd),
    }
}
