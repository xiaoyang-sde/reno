use clap::{Parser, Subcommand};
use serde_json::to_string;

use crate::container::fork_container;
use crate::state::Status;
use crate::{error::RuntimeError, state::State};
use oci_spec::runtime::Spec;
use std::path::Path;

const OCI_IMPL_ROOT: &str = "/tmp/oci-impl";

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

pub fn state(id: &String) -> Result<(), RuntimeError> {
    let container_root_path = Path::new(OCI_IMPL_ROOT).join(id);
    let state = State::load(&container_root_path)?;
    let serialized_state = to_string(&state).map_err(|err| RuntimeError {
        message: format!("failed to serialize the state: {}", err)
    })?;
    print!("{}", serialized_state);
    Ok(())
}

pub fn create(id: &String, bundle: &String) -> Result<(), RuntimeError> {
    let bundle = Path::new(bundle);
    bundle.try_exists().map_err(|err| RuntimeError {
        message: format!("the bundle doesn't exist: {}", err),
    })?;

    let spec = Spec::load(bundle).map_err(|err| RuntimeError {
        message: format!("failed to load the bundle configuration: {}", err),
    })?;

    let container_root_path = Path::new(OCI_IMPL_ROOT).join(id);
    let mut state = State::new(id.to_string(), bundle.to_path_buf());
    state.persist(&container_root_path)?;

    let container_socket_path = container_root_path.join("container.sock");

    let namespaces = match &spec.linux() {
        Some(linux) => linux.namespaces().clone().unwrap_or_default(),
        None => Vec::new(),
    };

    let pid = fork_container(&spec, &state, &namespaces, &container_socket_path)?;
    state.pid = pid.as_raw();
    state.status = Status::Created;
    state.persist(&container_root_path)?;

    Ok(())
}
