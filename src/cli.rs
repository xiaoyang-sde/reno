use clap::{Parser, Subcommand};
use serde_json::to_string;

use crate::container::fork_container;
use crate::socket::{SocketClient, SocketServer};
use crate::state::Status;
use crate::{error::RuntimeError, state::State};
use oci_spec::runtime::Spec;
use std::fs::create_dir_all;
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
        message: format!("failed to serialize the state: {}", err),
    })?;
    println!("{}", serialized_state);
    Ok(())
}

pub fn create(id: &String, bundle: &String) -> Result<(), RuntimeError> {
    let bundle = Path::new(bundle);
    bundle.try_exists().map_err(|err| RuntimeError {
        message: format!("the bundle doesn't exist: {}", err),
    })?;

    let bundle_spec = bundle.join("config.json");
    let spec = Spec::load(bundle_spec).map_err(|err| RuntimeError {
        message: format!("failed to load the bundle configuration: {}", err),
    })?;

    let container_root_path = Path::new(OCI_IMPL_ROOT).join(id);
    create_dir_all(&container_root_path).map_err(|err| RuntimeError {
        message: format!("failed to create the container root path: {}", err),
    })?;

    let mut state = State::new(id.to_string(), bundle.to_path_buf());
    state.persist(&container_root_path)?;

    let namespaces = match &spec.linux() {
        Some(linux) => linux.namespaces().clone().unwrap_or_default(),
        None => Vec::new(),
    };

    let init_socket_path = container_root_path.join("init.sock");
    let mut init_socket_server = SocketServer::bind(init_socket_path.to_path_buf()).unwrap();

    let container_socket_path = container_root_path.join("container.sock");
    let pid = fork_container(
        &spec,
        &state,
        &namespaces,
        &init_socket_path,
        &container_socket_path,
    )?;

    init_socket_server.listen().unwrap();
    let mut container_socket_client = SocketClient::connect(container_socket_path)?;

    let container_message = container_socket_client.read()?;
    if container_message == "created\n" {
        state.pid = pid.as_raw();
        state.status = Status::Created;
        state.persist(&container_root_path)?;
        Ok(())
    } else {
        Err(RuntimeError {
            message: format!("failed to create the container: {}", container_message),
        })
    }
}
