use std::os::unix::prelude::AsRawFd;

use crate::error::RuntimeError;

use nix::{
    fcntl::{open, OFlag},
    sched::setns,
    sched::CloneFlags,
    sys::stat::Mode,
};
use oci_spec::runtime::{LinuxNamespace, LinuxNamespaceType};

/// `namespace_to_clone_flag` converts a [LinuxNamespace] to [CloneFlags].
/// For more information, see the [clone(2)](https://man7.org/linux/man-pages/man2/clone.2.html)
/// man page.
pub fn namespace_to_clone_flag(namespace: &LinuxNamespace) -> CloneFlags {
    match namespace.typ() {
        LinuxNamespaceType::Mount => CloneFlags::CLONE_NEWNS,
        LinuxNamespaceType::Cgroup => CloneFlags::CLONE_NEWCGROUP,
        LinuxNamespaceType::Uts => CloneFlags::CLONE_NEWUTS,
        LinuxNamespaceType::Ipc => CloneFlags::CLONE_NEWIPC,
        LinuxNamespaceType::User => CloneFlags::CLONE_NEWUSER,
        LinuxNamespaceType::Pid => CloneFlags::CLONE_NEWPID,
        LinuxNamespaceType::Network => CloneFlags::CLONE_NEWNET,
    }
}

/// `set_namespace` moves the container process into namespaces associated with different paths.
/// For more information, see the [setns(2)](https://man7.org/linux/man-pages/man2/setns.2.html)
/// man page.
pub fn set_namespace(namespace_list: &Vec<LinuxNamespace>) -> Result<(), RuntimeError> {
    for namespace in namespace_list {
        if let Some(path) = namespace.path() {
            let fd = match open(path.as_os_str(), OFlag::empty(), Mode::empty()) {
                Ok(fd) => fd,
                Err(err) => return Err(RuntimeError::new(err.to_string())),
            };

            if let Err(err) = setns(fd.as_raw_fd(), namespace_to_clone_flag(namespace)) {
                return Err(RuntimeError::new(err.to_string()));
            }
        }
    }
    Ok(())
}
