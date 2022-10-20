use clap::Parser;

mod cli;
mod container;
mod device;
mod error;
mod mount;
mod process;
mod socket;
mod state;

use crate::cli::{create, delete, kill, start, state, OCISubcommand, OCI};
use crate::error::RuntimeError;

fn main() -> Result<(), RuntimeError> {
    let cli = OCI::parse();
    match &cli.command {
        OCISubcommand::State { id } => state(id)?,
        OCISubcommand::Create { id, bundle } => create(id, bundle)?,
        OCISubcommand::Start { id } => start(id)?,
        OCISubcommand::Kill { id, signal } => kill(id, signal)?,
        OCISubcommand::Delete { id } => delete(id)?,
    }

    Ok(())
}
