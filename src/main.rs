use clap::Parser;

mod cli;
mod container;
mod device;
mod error;
mod mount;
mod socket;
mod state;

use crate::cli::{create, OCI};
use crate::error::RuntimeError;

fn main() -> Result<(), RuntimeError> {
    let cli = OCI::parse();
    match &cli.command {
        cli::OCISubcommand::State { id: _ } => todo!(),
        cli::OCISubcommand::Create { id, bundle } => {
            create(id, bundle)?;
        }
        cli::OCISubcommand::Start { id: _ } => todo!(),
        cli::OCISubcommand::Kill { id: _ } => todo!(),
        cli::OCISubcommand::Delete { id: _ } => todo!(),
    }

    Ok(())
}
