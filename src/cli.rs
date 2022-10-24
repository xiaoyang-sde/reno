use clap::{Parser, Subcommand};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use serde_json::to_string;

use crate::container::fork_container;
use crate::socket::{SocketClient, SocketServer};
use crate::state::Status;
use crate::{error::RuntimeError, state::State};
use nix::sys::signal::{self, Signal};
use oci_spec::runtime::Spec;
use std::fs::{create_dir_all, remove_dir_all};
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
    Create {
        id: String,

        #[clap(long)]
        bundle: String,
    },

    #[clap(about = "start a container")]
    Start { id: String },

    #[clap(about = "kill a container")]
    Kill { id: String, signal: String },

    #[clap(about = "delete a container")]
    Delete { id: String },
}

pub fn state(id: &str) -> Result<(), RuntimeError> {
    let container_root = Path::new(OCI_IMPL_ROOT).join(id);
    let state = State::load(&container_root)?;
    let serialized_state = to_string(&state).map_err(|err| RuntimeError {
        message: format!("failed to serialize the state: {}", err),
    })?;
    println!("{}", serialized_state);
    Ok(())
}

pub fn create(id: &str, bundle: &str) -> Result<(), RuntimeError> {
    let bundle = Path::new(bundle);
    let bundle_exists = bundle.try_exists().map_err(|err| RuntimeError {
        message: format!("failed to check if the bundle exists: {}", err),
    })?;
    if !bundle_exists {
        return Err(RuntimeError { message: String::from("the bundle doesn't exist") });
    }

    let bundle_spec = bundle.join("config.json");
    let spec = Spec::load(bundle_spec).map_err(|err| RuntimeError {
        message: format!("failed to load the bundle configuration: {}", err),
    })?;

    let container_root = Path::new(OCI_IMPL_ROOT).join(id);
    let container_root_exists = container_root.try_exists().map_err(|err| RuntimeError {
        message: format!("failed to check if the container exists: {}", err),
    })?;
    if container_root_exists {
        return Err(RuntimeError { message: String::from("the container exists") });
    }

    create_dir_all(&container_root).map_err(|err| RuntimeError {
        message: format!("failed to create the container root path: {}", err),
    })?;

    let mut state = State::new(id.to_string(), bundle.to_path_buf());
    state.persist(&container_root)?;

    let namespaces = match &spec.linux() {
        Some(linux) => linux.namespaces().clone().unwrap_or_default(),
        None => Vec::new(),
    };

    let init_socket_path = container_root.join("init.sock");
    let mut init_socket_server = SocketServer::bind(&init_socket_path).unwrap();

    let container_socket_path = container_root.join("container.sock");
    let pid = fork_container(
        &spec,
        &state,
        &namespaces,
        &init_socket_path,
        &container_socket_path,
    )?;

    init_socket_server.listen().unwrap();
    let mut container_socket_client = SocketClient::connect(&container_socket_path)?;

    let container_message = container_socket_client.read()?;
    if container_message.status == Status::Created {
        state.pid = pid.as_raw();
        state.status = Status::Created;
        state.persist(&container_root)?;
        Ok(())
    } else if let Some(err) = container_message.error {
        Err(RuntimeError {
            message: format!("failed to create the container: {}", err),
        })
    } else {
        Err(RuntimeError {
            message: "failed to create the container".to_string(),
        })
    }
}

pub fn start(id: &str) -> Result<(), RuntimeError> {
    let container_root = Path::new(OCI_IMPL_ROOT).join(id);
    container_root
        .try_exists()
        .map_err(|err| RuntimeError {
            message: format!("the container doesn't exist: {}", err),
        })?;

    let mut state = State::load(&container_root)?;
    if state.status != Status::Created {
        return Err(RuntimeError {
            message: "the container is not in the 'Created' state".to_string(),
        });
    }

    let container_socket_path = container_root.join("container.sock");
    let mut container_socket_client = SocketClient::connect(&container_socket_path)?;

    let container_message = container_socket_client.read()?;
    if container_message.status == Status::Running {
        state.status = Status::Running;
        state.persist(&container_root)?;
        Ok(())
    } else if let Some(err) = container_message.error {
        Err(RuntimeError {
            message: format!("failed to start the container: {}", err),
        })
    } else {
        Err(RuntimeError {
            message: "failed to start the container".to_string(),
        })
    }
}

pub fn kill(id: &str, signal: &str) -> Result<(), RuntimeError> {
    let container_root = Path::new(OCI_IMPL_ROOT).join(id);
    container_root
        .try_exists()
        .map_err(|err| RuntimeError {
            message: format!("the container doesn't exist: {}", err),
        })?;

    let mut state = State::load(&container_root)?;
    if state.status != Status::Created && state.status != Status::Running {
        return Err(RuntimeError {
            message: "the container is not in the 'Created' or 'Running' state".to_string(),
        });
    }

    let signal = match signal {
        "HUP" => Signal::SIGHUP,
        "INT" => Signal::SIGINT,
        "TERM" => Signal::SIGTERM,
        "STOP" => Signal::SIGSTOP,
        "KILL" => Signal::SIGKILL,
        _ => Signal::SIGKILL,
    };

    let pid = Pid::from_raw(state.pid);
    signal::kill(pid, signal).map_err(|err| RuntimeError {
        message: format!("failed to kill the container: {}", err),
    })?;

    let wait_status = waitpid(pid, Some(WaitPidFlag::WNOHANG)).map_err(|err| RuntimeError {
        message: format!("failed to get the WaitStatus of the container: {}", err),
    })?;

    match wait_status {
        WaitStatus::Exited(_, _) | WaitStatus::Signaled(_, _, _) => {
            state.status = Status::Stopped;
            state.persist(&container_root)?;
        }
        _ => (),
    }

    Ok(())
}

pub fn delete(id: &str) -> Result<(), RuntimeError> {
    let container_root = Path::new(OCI_IMPL_ROOT).join(id);
    container_root
        .try_exists()
        .map_err(|err| RuntimeError {
            message: format!("the container doesn't exist: {}", err),
        })?;

    let state = State::load(&container_root)?;
    if state.status != Status::Stopped {
        return Err(RuntimeError {
            message: "the container is not in the 'Stopped' state".to_string(),
        });
    }

    remove_dir_all(container_root).map_err(|err| RuntimeError {
        message: format!("failed to remove the container: {}", err),
    })?;
    Ok(())
}
