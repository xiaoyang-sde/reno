use std::fs::write;

use std::{ffi::CString, os::unix::prelude::AsRawFd, path::Path, process::exit};

use crate::cap::set_cap;
use crate::{
    device::{create_default_device, create_default_symlink, create_device},
    error::RuntimeError,
    hook::run_hook,
    mount::{mount_rootfs, oci_mount, pivot_rootfs},
    process::clone_child,
    socket::{SocketClient, SocketMessage, SocketServer},
    state::{State, Status},
};
use caps::CapSet;
use nix::unistd::setgid;
use nix::unistd::setuid;
use nix::unistd::Gid;
use nix::unistd::Uid;
use nix::{
    fcntl::{open, OFlag},
    sched::setns,
    sched::CloneFlags,
    sys::stat::Mode,
    unistd::{chdir, execvp, sethostname, Pid},
};
use oci_spec::runtime::{LinuxNamespace, Spec};
use std::env::set_var;

pub fn fork_container(
    spec: &Spec,
    state: &State,
    namespaces: &Vec<LinuxNamespace>,
    init_socket_path: &Path,
    container_socket_path: &Path,
) -> Result<Pid, RuntimeError> {
    clone_child(
        || {
            let mut container_socket_server = SocketServer::bind(container_socket_path).unwrap();
            let init_socket_client = SocketClient::connect(init_socket_path).unwrap();
            init_socket_client.shutdown().unwrap();
            container_socket_server.listen().unwrap();

            for namespace in namespaces {
                if let Some(path) = namespace.path() {
                    let fd = match open(path.as_os_str(), OFlag::empty(), Mode::empty()) {
                        Ok(fd) => fd,
                        Err(err) => {
                            container_socket_server
                                .write(SocketMessage {
                                    status: Status::Creating,
                                    error: Some(RuntimeError {
                                        message: format!("container error: {}", err),
                                    }),
                                })
                                .unwrap();
                            exit(1);
                        }
                    };

                    if let Err(err) = setns(fd.as_raw_fd(), CloneFlags::empty()) {
                        container_socket_server
                            .write(SocketMessage {
                                status: Status::Creating,
                                error: Some(RuntimeError {
                                    message: format!("container error: {}", err),
                                }),
                            })
                            .unwrap();
                        exit(1);
                    }
                }
            }

            let rootfs = &state.bundle.join(spec.root().as_ref().unwrap().path());
            if let Err(err) = mount_rootfs(rootfs) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Creating,
                        error: Some(RuntimeError {
                            message: format!("container error: {}", err),
                        }),
                    })
                    .unwrap();
                exit(1);
            }

            if let Some(mounts) = &spec.mounts() {
                for mount in mounts {
                    if let Err(err) = oci_mount(rootfs, mount) {
                        container_socket_server
                            .write(SocketMessage {
                                status: Status::Creating,
                                error: Some(RuntimeError {
                                    message: format!("container error: {}", err),
                                }),
                            })
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
                                .write(SocketMessage {
                                    status: Status::Creating,
                                    error: Some(RuntimeError {
                                        message: format!("container error: {}", err),
                                    }),
                                })
                                .unwrap();
                            exit(1);
                        }
                    }
                }
            }

            create_default_device(rootfs);
            if let Err(err) = create_default_symlink(rootfs) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Creating,
                        error: Some(RuntimeError {
                            message: format!("container error: {}", err),
                        }),
                    })
                    .unwrap();
                exit(1);
            }

            if let Some(hostname) = spec.hostname() {
                sethostname(hostname).unwrap();
            }

            container_socket_server
                .write(SocketMessage {
                    status: Status::Creating,
                    error: None,
                })
                .unwrap();
            container_socket_server.listen().unwrap();

            if let Some(hooks) = spec.hooks() {
                if let Some(create_container_hooks) = hooks.create_container() {
                    for create_container_hook in create_container_hooks {
                        if let Err(err) = run_hook(state, create_container_hook) {
                            container_socket_server
                                .write(SocketMessage {
                                    status: Status::Stopped,
                                    error: Some(RuntimeError {
                                        message: format!("container error: {}", err),
                                    }),
                                })
                                .unwrap();
                            exit(1);
                        }
                    }
                }
            }

            if let Err(err) = pivot_rootfs(rootfs) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Creating,
                        error: Some(RuntimeError {
                            message: format!("container error: {}", err),
                        }),
                    })
                    .unwrap();
                exit(1);
            }

            if let Some(linux) = spec.linux() {
                if let Some(sysctl) = linux.sysctl() {
                    for (field, value) in sysctl {
                        let sysctl_path = Path::new("/proc/sys").join(field.replace('.', "/"));
                        if let Err(err) = write(sysctl_path, value) {
                            container_socket_server
                                .write(SocketMessage {
                                    status: Status::Stopped,
                                    error: Some(RuntimeError {
                                        message: format!(
                                            "failed to set {} to {}: {}",
                                            field, value, err
                                        ),
                                    }),
                                })
                                .unwrap();
                            exit(1);
                        }
                    }
                }
            }

            container_socket_server
                .write(SocketMessage {
                    status: Status::Created,
                    error: None,
                })
                .unwrap();
            container_socket_server.listen().unwrap();

            if let Some(hooks) = spec.hooks() {
                if let Some(start_container_hooks) = hooks.start_container() {
                    for start_container_hook in start_container_hooks {
                        if let Err(err) = run_hook(state, start_container_hook) {
                            container_socket_server
                                .write(SocketMessage {
                                    status: Status::Stopped,
                                    error: Some(RuntimeError {
                                        message: format!("container error: {}", err),
                                    }),
                                })
                                .unwrap();
                            exit(1);
                        }
                    }
                }
            }

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

                if let Some(capabilities) = process.capabilities() {
                    if let Some(ambient_cap_set) = capabilities.ambient() {
                        for ambient_cap in ambient_cap_set {
                            if let Err(err) = set_cap(CapSet::Ambient, ambient_cap) {
                                println!("container error: {}", err);
                            }
                        }
                    }

                    if let Some(effective_cap_set) = capabilities.effective() {
                        for effective_cap in effective_cap_set {
                            if let Err(err) = set_cap(CapSet::Effective, effective_cap) {
                                println!("container error: {}", err);
                            }
                        }
                    }

                    if let Some(inheritable_cap_set) = capabilities.inheritable() {
                        for inheritable_cap in inheritable_cap_set {
                            if let Err(err) = set_cap(CapSet::Inheritable, inheritable_cap) {
                                println!("container error: {}", err);
                            }
                        }
                    }

                    if let Some(permitted_cap_set) = capabilities.permitted() {
                        for permitted_cap in permitted_cap_set {
                            if let Err(err) = set_cap(CapSet::Permitted, permitted_cap) {
                                println!("container error: {}", err);
                            }
                        }
                    }

                    if let Some(bounding_cap_set) = capabilities.bounding() {
                        for bounding_cap in bounding_cap_set {
                            if let Err(err) = set_cap(CapSet::Bounding, bounding_cap) {
                                println!("container error: {}", err);
                            }
                        }
                    }
                }

                chdir(process.cwd()).unwrap();

                container_socket_server
                    .write(SocketMessage {
                        status: Status::Running,
                        error: None,
                    })
                    .unwrap();

                if let Err(err) = execvp(&command, &arguments) {
                    container_socket_server
                        .write(SocketMessage {
                            status: Status::Stopped,
                            error: Some(RuntimeError {
                                message: format!("container error: {}", err),
                            }),
                        })
                        .unwrap();
                    exit(1);
                }
            } else {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Stopped,
                        error: Some(RuntimeError {
                            message: "container error: the 'process' doesn't exist".to_string(),
                        }),
                    })
                    .unwrap();
                exit(1);
            }

            0
        },
        namespaces,
    )
}
