use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct OCI {
    #[clap(subcommand)]
    pub command: OCISubcommand,
}

#[derive(Subcommand, Debug)]
pub enum OCISubcommand {
    #[clap(about = "print the state of a container")]
    State { id: String },

    #[clap(about = "create a container")]
    Create { id: String, bundle: String },

    #[clap(about = "start a container")]
    Start { id: String },

    #[clap(about = "kill a container")]
    Kill { id: String },

    #[clap(about = "delete a container")]
    Delete { id: String },
}
