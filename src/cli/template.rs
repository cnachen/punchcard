//! Template discovery commands (`punch template ...`).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use punchcard::TemplateRegistry;

/// Template subcommands.
#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// List all known templates.
    List,
    /// Show column rules for a template.
    Show(TemplateShowArgs),
}

/// Arguments for `punch template show`.
#[derive(Args, Debug)]
pub struct TemplateShowArgs {
    /// Template name to display.
    pub name: String,
}

/// Execute a template command.
pub fn handle(command: TemplateCommand) -> Result<()> {
    match command {
        TemplateCommand::List => list(),
        TemplateCommand::Show(args) => show(args),
    }
}

fn list() -> Result<()> {
    println!("Available templates:");
    for tpl in TemplateRegistry::list() {
        println!("  - {}: {}", tpl.name, tpl.description);
    }
    Ok(())
}

fn show(args: TemplateShowArgs) -> Result<()> {
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
    Ok(())
}
