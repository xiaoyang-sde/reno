use std::{ffi::CString, path::Path, process::exit};

use anyhow::{bail, Result};
use nix::unistd::{self, Pid};
use oci_spec::runtime::{LinuxNamespace, Spec};

use crate::{
    container::{create, start},
    linux::process,
    socket::{SocketClient, SocketMessage, SocketServer},
    state::{State, Status},
};

/// `pipeline` initializes the container environment, run hooks, and start the container process.
/// The pipeline contains these phases:
/// - [init_environment](create::init_environment): Mount the root file system, create devices and symbolic links, and change the hostname
/// - Listen on the `container_socket_server` to wait the runtime to invoke the `create_runtime` hook
/// - [create_container](create::create_container): Run the `create_container` hook, change the root mount, and change kernel parameters
/// - Listen on the `container_socket_server` to wait the runtime to invoke the `prestart` hook
/// - [start_container](start::start_container): Run the `start_container` hook, set resource limits, capabilities, and ownership of the container process
/// - [execvp](unistd::execvp): Start the container process
pub fn pipeline(
    spec: &Spec,
    state: &State,
    namespace_list: &[LinuxNamespace],
    container_socket_server: &mut SocketServer,
) -> Result<()> {
    create::init_environment(spec, state, namespace_list)?;
    container_socket_server.write(SocketMessage::new(Status::Creating, None))?;

    // Listen on the `container_socket_server` to wait the runtime to invoke the `create_runtime` hook
    container_socket_server.listen()?;
    create::create_container(spec, state)?;
    container_socket_server.write(SocketMessage::new(Status::Created, None))?;

    // Listen on the `container_socket_server` to wait the runtime to invoke the `prestart` hook
    container_socket_server.listen().unwrap();
    start::start_container(spec, state)?;
    container_socket_server.write(SocketMessage::new(Status::Running, None))?;

    if let Some(process) = spec.process() {
        let command = CString::new(process.args().as_ref().unwrap()[0].as_bytes())?;
        let argument_list: Vec<CString> = process
            .args()
            .as_ref()
            .unwrap()
            .iter()
            .map(|a| CString::new(a.to_string()).unwrap_or_default())
            .collect();

        unistd::execvp(&command, &argument_list)?;
    } else {
        bail!("the 'process' field doesn't exist");
    }

    Ok(())
}

/// `fork_container` clones a new process that invokes the [pipeline] function,
/// which initializes the container environment, run hooks, and start the container process.
pub fn fork_container(
    spec: &Spec,
    state: &State,
    namespace_list: &[LinuxNamespace],
    init_socket_path: &Path,
    container_socket_path: &Path,
) -> Result<Pid> {
    process::clone_child(namespace_list, || {
        // Initialize the `container_socket_server` that enables communication between
        // the container process and the `reno` CLI
        let mut container_socket_server = SocketServer::bind(container_socket_path).unwrap();

        // Connect to the socket server on `init_socket_path` to let the `reno` CLI know that
        // the `container_socket_server` is initialized
        let init_socket_client = SocketClient::connect(init_socket_path).unwrap();
        init_socket_client.shutdown().unwrap();

        // Wait for the `reno` CLI to connect to the `container_socket_server`
        container_socket_server.listen().unwrap();

        if let Err(error) = pipeline(spec, state, namespace_list, &mut container_socket_server) {
            container_socket_server
                .write(SocketMessage::new(Status::Stopped, Some(error.to_string())))
                .unwrap();
            exit(1);
        }

        0
    })
}
