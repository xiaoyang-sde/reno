use clap::Parser;

mod cap;
mod cli;
mod container;
mod device;
mod error;
mod hook;
mod linux;
mod mount;
mod process;
mod rlimit;
mod socket;
mod state;

use crate::cli::{create, delete, kill, start, state, Cli, CliSubcommand};
use crate::error::RuntimeError;

fn main() -> Result<(), RuntimeError> {
    let cli = Cli::parse();
    match &cli.command {
        CliSubcommand::State { id } => state(id)?,
        CliSubcommand::Create {
            id,
            bundle,
            pid_file,
        } => create(id, bundle, pid_file)?,
        CliSubcommand::Start { id } => start(id)?,
        CliSubcommand::Kill { id, signal } => kill(id, signal)?,
        CliSubcommand::Delete { id } => delete(id)?,
    }

    Ok(())
}
