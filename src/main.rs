use clap::Parser;

mod cli;
mod container;
mod device;
mod error;
mod mount;
mod socket;
mod state;

use crate::cli::{create, state, OCI, OCISubcommand};
use crate::error::RuntimeError;

fn main() -> Result<(), RuntimeError> {
    let cli = OCI::parse();
    match &cli.command {
        OCISubcommand::State { id } => state(id)?,
        OCISubcommand::Create { id, bundle } => create(id, bundle)?,
        OCISubcommand::Start { id: _ } => todo!(),
        OCISubcommand::Kill { id: _ } => todo!(),
        OCISubcommand::Delete { id: _ } => todo!(),
    }

    Ok(())
}
