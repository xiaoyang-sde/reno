use std::fs::write;

use std::{ffi::CString, path::Path, process::exit};

use crate::linux::cap::set_cap;
use crate::linux::hostname::set_hostname;
use crate::linux::namespace::set_namespace;
use crate::linux::rlimit::set_rlimit;
use crate::linux::sysctl::set_sysctl;
use crate::{
    error::RuntimeError,
    hook::run_hook,
    linux::device::{create_default_device, create_default_symlink, create_device},
    linux::mount::{custom_mount, mount_rootfs, pivot_rootfs},
    linux::process::clone_child,
    socket::{SocketClient, SocketMessage, SocketServer},
    state::{State, Status},
};
use caps::CapSet;

use nix::sys::stat::umask;
use nix::unistd::setuid;
use nix::unistd::Gid;
use nix::unistd::Uid;
use nix::unistd::{setgid, setgroups};
use nix::{
    sys::stat::Mode,
    unistd::{chdir, execvp, Pid},
};
use oci_spec::runtime::{LinuxNamespace, Spec};
use prctl::set_keep_capabilities;
use std::env::set_var;

fn init_container(
    spec: &Spec,
    state: &State,
    namespace_list: &Vec<LinuxNamespace>,
) -> Result<(), RuntimeError> {
    set_namespace(namespace_list)?;

    let rootfs = &state.bundle.join(spec.root().as_ref().unwrap().path());
    mount_rootfs(rootfs)?;

    if let Some(mounts) = &spec.mounts() {
        for mount in mounts {
            custom_mount(rootfs, mount)?;
        }
    }

    if let Some(linux) = spec.linux() {
        if let Some(devices) = linux.devices() {
            for device in devices {
                create_device(rootfs, device)?;
            }
        }
    }

    create_default_device(rootfs)?;
    create_default_symlink(rootfs)?;

    if let Some(hostname) = spec.hostname() {
        set_hostname(hostname)?;
    }

    Ok(())
}

fn create_container(spec: &Spec, state: &State) -> Result<(), RuntimeError> {
    if let Some(hooks) = spec.hooks() {
        if let Some(create_container_hooks) = hooks.create_container() {
            for create_container_hook in create_container_hooks {
                run_hook(state, create_container_hook)?;
            }
        }
    }

    let rootfs = &state.bundle.join(spec.root().as_ref().unwrap().path());
    pivot_rootfs(rootfs)?;

    if let Some(linux) = spec.linux() {
        if let Some(sysctl) = linux.sysctl() {
            set_sysctl(sysctl)?;
        }
    }
    Ok(())
}

fn start_container(spec: &Spec, state: &State) -> Result<(), RuntimeError> {
    if let Some(hooks) = spec.hooks() {
        if let Some(start_container_hooks) = hooks.start_container() {
            for start_container_hook in start_container_hooks {
                run_hook(state, start_container_hook)?;
            }
        }
    }

    if let Some(process) = spec.process() {
        if let Some(env_list) = process.env() {
            for env in env_list {
                if let Some((k, v)) = env.split_once('=') {
                    set_var(k, v);
                }
            }
        }

        if let Some(rlimits) = process.rlimits() {
            for rlimit in rlimits {
                set_rlimit(rlimit)?;
            }
        }

        if let Some(oom_score_adj) = process.oom_score_adj() {
            let sysctl_path = Path::new("/proc/self/oom_score_adj");
            write(sysctl_path, oom_score_adj.to_string()).map_err(|err| {
                RuntimeError::new(format!(
                    "failed to set oom_score_adj to {}: {}",
                    oom_score_adj, err
                ))
            })?;
        }

        if let Some(capabilities) = process.capabilities() {
            if let Some(capabilities) = capabilities.bounding() {
                set_cap(CapSet::Bounding, capabilities)?;
            }
        }

        set_keep_capabilities(true).map_err(|err| {
            RuntimeError::new(format!("failed to set PR_SET_KEEPCAPS to true: {}", err))
        })?;
        setgid(Gid::from_raw(process.user().gid())).map_err(|err| {
            RuntimeError::new(format!(
                "failed to set gid to {}: {}",
                process.user().gid(),
                err
            ))
        })?;

        if let Some(mode) = process.user().umask() {
            if let Some(mode) = Mode::from_bits(mode) {
                umask(mode);
            } else {
                return Err(RuntimeError::new(format!("invalid umask: {}", mode)));
            }
        }

        if let Some(additional_gids) = process.user().additional_gids() {
            let additional_gids: &Vec<Gid> = &additional_gids
                .iter()
                .map(|gid| Gid::from_raw(*gid))
                .collect();
            setgroups(additional_gids).map_err(|err| {
                RuntimeError::new(format!("failed to set additional gids: {}", err))
            })?;
        }
        setuid(Uid::from_raw(process.user().uid())).map_err(|err| {
            RuntimeError::new(format!(
                "failed to set uid to {}: {}",
                process.user().uid(),
                err
            ))
        })?;

        set_keep_capabilities(false).map_err(|err| {
            RuntimeError::new(format!("failed to set PR_SET_KEEPCAPS to false: {}", err))
        })?;

        if let Some(capabilities) = process.capabilities() {
            let capabilities_list = [
                (capabilities.effective(), CapSet::Effective),
                (capabilities.permitted(), CapSet::Permitted),
                (capabilities.inheritable(), CapSet::Inheritable),
                (capabilities.ambient(), CapSet::Ambient),
            ];
            for (capabilities, capabilities_set_flag) in capabilities_list.into_iter() {
                if let Some(capabilities) = capabilities {
                    if let Err(err) = set_cap(capabilities_set_flag, capabilities) {
                        println!("container error: {}", err);
                    }
                }
            }
        }

        chdir(process.cwd()).map_err(|err| {
            RuntimeError::new(format!(
                "failed to change the working directory to {}: {}",
                process.cwd().display(),
                err
            ))
        })?;

        Ok(())
    } else {
        Err(RuntimeError {
            message: "container error: the 'process' doesn't exist".to_string(),
        })
    }
}

pub fn fork_container(
    spec: &Spec,
    state: &State,
    namespace_list: &Vec<LinuxNamespace>,
    init_socket_path: &Path,
    container_socket_path: &Path,
) -> Result<Pid, RuntimeError> {
    clone_child(
        || {
            let mut container_socket_server = SocketServer::bind(container_socket_path).unwrap();
            let init_socket_client = SocketClient::connect(init_socket_path).unwrap();
            init_socket_client.shutdown().unwrap();
            container_socket_server.listen().unwrap();

            if let Err(err) = init_container(spec, state, namespace_list) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Creating,
                        error: Some(err),
                    })
                    .unwrap();
                exit(1);
            }

            container_socket_server
                .write(SocketMessage {
                    status: Status::Creating,
                    error: None,
                })
                .unwrap();
            container_socket_server.listen().unwrap();

            if let Err(err) = create_container(spec, state) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Stopped,
                        error: Some(err),
                    })
                    .unwrap();
                exit(1);
            }

            container_socket_server
                .write(SocketMessage {
                    status: Status::Created,
                    error: None,
                })
                .unwrap();
            container_socket_server.listen().unwrap();

            if let Err(err) = start_container(spec, state) {
                container_socket_server
                    .write(SocketMessage {
                        status: Status::Stopped,
                        error: Some(err),
                    })
                    .unwrap();
                exit(1);
            }

            container_socket_server
                .write(SocketMessage {
                    status: Status::Running,
                    error: None,
                })
                .unwrap();

            if let Some(process) = spec.process() {
                let command = CString::new(process.args().as_ref().unwrap()[0].as_bytes()).unwrap();
                let arguments: Vec<CString> = process
                    .args()
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|a| CString::new(a.to_string()).unwrap_or_default())
                    .collect();

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
            }

            0
        },
        namespace_list,
    )
}
