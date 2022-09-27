use clap::Parser;
mod cli;

use crate::cli::OCI;

fn main() {
    let _cli = OCI::parse();
    println!("Hello, world!");
}
