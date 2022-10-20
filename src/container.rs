use std::{ffi::CString, path::Path, process::exit};

use crate::{
    device::{create_default_device, create_default_symlink, create_device},
    error::RuntimeError,
    mount::{mount_rootfs, oci_mount, pivot_rootfs},
    socket::{SocketClient, SocketServer},
    state::State,
};
use nix::unistd::setgid;
use nix::unistd::setuid;
use nix::unistd::Gid;
use nix::unistd::Uid;
use nix::{
    fcntl::{open, OFlag},
    sched::CloneFlags,
    sched::{clone, setns},
    sys::stat::Mode,
    unistd::{chdir, execvp, sethostname, Pid},
};
use oci_spec::runtime::{LinuxNamespace, LinuxNamespaceType, Spec};
use std::env::set_var;
use std::os::unix::io::AsRawFd;

fn clone_child(
    child_fn: impl FnMut() -> isize,
    namespaces: &[LinuxNamespace],
) -> Result<Pid, RuntimeError> {
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let mut stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

    let clone_flags = namespaces
        .iter()
        .map(|namespace| match namespace.typ() {
            LinuxNamespaceType::Mount => CloneFlags::CLONE_NEWNS,
            LinuxNamespaceType::Cgroup => CloneFlags::CLONE_NEWCGROUP,
            LinuxNamespaceType::Uts => CloneFlags::CLONE_NEWUTS,
            LinuxNamespaceType::Ipc => CloneFlags::CLONE_NEWIPC,
            LinuxNamespaceType::User => CloneFlags::CLONE_NEWUSER,
            LinuxNamespaceType::Pid => CloneFlags::CLONE_NEWPID,
            LinuxNamespaceType::Network => CloneFlags::CLONE_NEWNET,
        })
        .reduce(|flag_1, flag_2| flag_1 | flag_2)
        .unwrap_or(CloneFlags::empty());

    let pid =
        clone(Box::new(child_fn), &mut stack, clone_flags, None).map_err(|err| RuntimeError {
            message: format!("failed to invoke clone(): {}", err),
        })?;

    Ok(pid)
}

pub fn fork_container(
    spec: &Spec,
    _state: &State,
    namespaces: &Vec<LinuxNamespace>,
    init_socket_path: &Path,
    container_socket_path: &Path,
) -> Result<Pid, RuntimeError> {
    clone_child(
        || {
            let mut container_socket_server =
                SocketServer::bind(container_socket_path.to_path_buf()).unwrap();
            let init_socket_client = SocketClient::connect(init_socket_path.to_path_buf()).unwrap();
            init_socket_client.shutdown().unwrap();
            container_socket_server.listen().unwrap();

            for namespace in namespaces {
                if let Some(path) = namespace.path() {
                    let fd = match open(path.as_os_str(), OFlag::empty(), Mode::empty()) {
                        Ok(fd) => fd,
                        Err(err) => {
                            container_socket_server
                                .write(format!("container error: {}", err))
                                .unwrap();
                            exit(1);
                        }
                    };

                    if let Err(err) = setns(fd.as_raw_fd(), CloneFlags::empty()) {
                        container_socket_server
                            .write(format!("container error: {}", err))
                            .unwrap();
                        exit(1);
                    }
                }
            }

            let rootfs = spec.root().as_ref().unwrap().path();

            if let Err(err) = mount_rootfs(rootfs) {
                container_socket_server
                    .write(format!("container error: {}\n", err))
                    .unwrap();
                exit(1);
            }

            if let Some(mounts) = &spec.mounts() {
                for mount in mounts {
                    if let Err(err) = oci_mount(rootfs, mount) {
                        container_socket_server
                            .write(format!("container error: {}\n", err))
                            .unwrap();
                        exit(1);
                    }
                }
            }

            if let Some(linux) = spec.linux() {
                if let Some(devices) = linux.devices() {
                    for device in devices {
                        if let Err(err) = create_device(rootfs, device) {
                            container_socket_server
                                .write(format!("container error: {}\n", err))
                                .unwrap();
                            exit(1);
                        }
                    }
                }
            }

            create_default_device(rootfs);
            if let Err(err) = create_default_symlink(rootfs) {
                container_socket_server
                    .write(format!("container error: {}", err))
                    .unwrap();
                exit(1);
            }

            if let Err(err) = pivot_rootfs(rootfs) {
                container_socket_server
                    .write(format!("container error: {}\n", err))
                    .unwrap();
                exit(1);
            }

            if let Some(hostname) = spec.hostname() {
                sethostname(hostname).unwrap();
            }

            container_socket_server
                .write("created\n".to_string())
                .unwrap();
            container_socket_server.listen().unwrap();

            if let Some(process) = spec.process() {
                let command = CString::new(process.args().as_ref().unwrap()[0].as_bytes()).unwrap();
                let arguments: Vec<CString> = process
                    .args()
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|a| CString::new(a.to_string()).unwrap_or_default())
                    .collect();

                if let Some(env_list) = process.env() {
                    for env in env_list {
                        if let Some((k, v)) = env.split_once('=') {
                            set_var(k, v);
                        }
                    }
                }

                setuid(Uid::from_raw(process.user().uid())).unwrap();
                setgid(Gid::from_raw(process.user().gid())).unwrap();

                chdir(process.cwd()).unwrap();
                if let Err(err) = execvp(&command, &arguments) {
                    container_socket_server
                        .write(format!("container error: {}", err))
                        .unwrap();
                    exit(1);
                }
            }

            0
        },
        namespaces,
    )
}
