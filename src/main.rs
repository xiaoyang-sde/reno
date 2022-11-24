use clap::Parser;

mod cli;
mod container;
mod hook;
mod linux;
mod socket;
mod state;

use crate::cli::{Cli, CliSubcommand};
use anyhow::Result;

fn main() -> Result<()> {
    match &Cli::parse().command {
        CliSubcommand::State { id } => cli::state(id),
        CliSubcommand::Create {
            id,
            bundle,
            pid_file,
        } => cli::create(id, bundle, pid_file),
        CliSubcommand::Start { id } => cli::start(id),
        CliSubcommand::Kill { id, signal } => cli::kill(id, signal),
        CliSubcommand::Delete { id } => cli::delete(id),
    }
}
