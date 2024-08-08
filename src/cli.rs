use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};
use oci_spec::runtime::Spec;

use crate::{
    container::fork,
    hook,
    socket::{SocketClient, SocketServer},
    state::{State, Status},
};

const RENO_ROOT: &str = "/tmp/reno";

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CliSubcommand {
    #[command(about = "print the state of a container")]
    State { id: String },

    #[command(about = "create a container")]
    Create {
        id: String,

        #[arg(long)]
        bundle: String,

        #[arg(long)]
        pid_file: Option<String>,
    },

    #[command(about = "start a container")]
    Start { id: String },

    #[command(about = "kill a container")]
    Kill { id: String, signal: String },

    #[command(about = "delete a container")]
    Delete { id: String },
}

pub fn state(id: &str) -> Result<()> {
    let container_root = Path::new(RENO_ROOT).join(id);
    let mut state = State::load(&container_root)?;
    state.refresh();

    let serialized_state =
        serde_json::to_string(&state).context("failed to serialize the state")?;
    println!("{}", serialized_state);

    state.persist(&container_root)?;
    Ok(())
}

pub fn create(id: &str, bundle: &str, pid_file: &Option<String>) -> Result<()> {
    let bundle = Path::new(bundle);
    let bundle_exists = bundle
        .try_exists()
        .context("failed to check if the bundle exists")?;
    if !bundle_exists {
        bail!("the bundle doesn't exist");
    }

    let bundle_spec = bundle.join("config.json");
    let spec = Spec::load(bundle_spec).context("failed to load the bundle configuration")?;

    let container_root = Path::new(RENO_ROOT).join(id);
    let container_root_exists = container_root
        .try_exists()
        .context("failed to check if the container exists")?;
    if container_root_exists {
        bail!("the container exists");
    }

    fs::create_dir_all(&container_root).context("failed to create the container root path")?;

    let mut state = State::new(id.to_string(), bundle.to_path_buf());
    state.persist(&container_root)?;

    let namespaces = match &spec.linux() {
        Some(linux) => linux.namespaces().clone().unwrap_or_default(),
        None => Vec::new(),
    };

    let init_socket_path = container_root.join("init.sock");
    let mut init_socket_server = SocketServer::bind(&init_socket_path)?;

    let container_socket_path = container_root.join("container.sock");
    let pid = fork::fork_container(
        &spec,
        &state,
        &namespaces,
        &init_socket_path,
        &container_socket_path,
    )?;

    init_socket_server.listen()?;

    let mut container_socket_client = SocketClient::connect(&container_socket_path)?;
    let container_message = container_socket_client.read()?;
    container_socket_client.shutdown()?;

    if container_message.status == Status::Creating {
        if let Some(hooks) = spec.hooks() {
            if let Some(create_runtime_hooks) = hooks.create_runtime() {
                for create_runtime_hook in create_runtime_hooks {
                    hook::run_hook(&state, create_runtime_hook)
                        .context("failed to invoke the create_runtime hook")?;
                }
            }
        }
    } else if let Some(error) = container_message.error {
        bail!("failed to create the container: {}", error);
    } else {
        bail!("failed to create the container");
    }

    let mut container_socket_client = SocketClient::connect(&container_socket_path)?;
    let container_message = container_socket_client.read()?;
    container_socket_client.shutdown()?;

    if container_message.status == Status::Created {
        state.pid = pid.as_raw();
        state.status = Status::Created;
        state.persist(&container_root)?;
        if let Some(pid_file) = pid_file {
            state.write_pid_file(Path::new(pid_file))?;
        }
        Ok(())
    } else if let Some(error) = container_message.error {
        bail!("failed to create the container: {}", error);
    } else {
        bail!("failed to create the container");
    }
}

pub fn start(id: &str) -> Result<()> {
    let container_root = Path::new(RENO_ROOT).join(id);
    container_root
        .try_exists()
        .context("the container doesn't exist")?;

    let mut state = State::load(&container_root)?;
    if state.status != Status::Created {
        bail!("the container is not in the 'Created' state");
    }

    let bundle_spec = state.bundle.join("config.json");
    let spec = Spec::load(bundle_spec).context("failed to load the bundle configuration")?;

    if let Some(hooks) = spec.hooks() {
        if let Some(pre_start_hooks) = hooks.prestart() {
            for pre_start_hook in pre_start_hooks {
                hook::run_hook(&state, pre_start_hook)
                    .context("failed to invoke the pre_start hook")?;
            }
        }
    }

    let container_socket_path = container_root.join("container.sock");
    let mut container_socket_client = SocketClient::connect(&container_socket_path)?;
    let container_message = container_socket_client.read()?;
    container_socket_client.shutdown()?;

    if container_message.status == Status::Running {
        state.refresh();
        state.persist(&container_root)?;

        if let Some(hooks) = spec.hooks() {
            if let Some(post_start_hooks) = hooks.poststart() {
                for post_start_hook in post_start_hooks {
                    hook::run_hook(&state, post_start_hook)
                        .context("failed to invoke the post_start hook")?;
                }
            }
        }
        Ok(())
    } else if let Some(error) = container_message.error {
        bail!("failed to start the container: {}", error);
    } else {
        bail!("failed to start the container");
    }
}

pub fn kill(id: &str, signal: &str) -> Result<()> {
    let container_root = Path::new(RENO_ROOT).join(id);
    container_root
        .try_exists()
        .context("the container doesn't exist")?;

    let mut state = State::load(&container_root)?;
    if state.status != Status::Created && state.status != Status::Running {
        bail!("the container is not in the 'Created' or 'Running' state");
    }

    let signal = match signal {
        "HUP" => Signal::SIGHUP,
        "INT" => Signal::SIGINT,
        "TERM" => Signal::SIGTERM,
        "STOP" => Signal::SIGSTOP,
        "KILL" => Signal::SIGKILL,
        "USR1" => Signal::SIGUSR1,
        "USR2" => Signal::SIGUSR2,
        _ => Signal::SIGKILL,
    };

    let pid = Pid::from_raw(state.pid);
    signal::kill(pid, signal).context("failed to kill the container")?;

    state.refresh();
    state.persist(&container_root)?;
    Ok(())
}

pub fn delete(id: &str) -> Result<()> {
    let container_root = Path::new(RENO_ROOT).join(id);
    container_root
        .try_exists()
        .context("the container doesn't exist")?;

    let state = State::load(&container_root)?;
    if state.status != Status::Stopped {
        bail!("the container is not in the 'Stopped' state");
    }

    fs::remove_dir_all(container_root).context("failed to remove the container")?;

    let bundle_spec = state.bundle.join("config.json");
    let spec = Spec::load(bundle_spec).context("failed to load the bundle configuration")?;
    if let Some(hooks) = spec.hooks() {
        if let Some(post_stop_hooks) = hooks.poststop() {
            for post_stop_hook in post_stop_hooks {
                hook::run_hook(&state, post_stop_hook)
                    .context("failed to invoke the post_stop hook")?;
            }
        }
    }
    Ok(())
}
