use std::os::unix::prelude::AsRawFd;

use anyhow::{Context, Result};
use nix::{
    fcntl::{self, OFlag},
    sched,
    sched::CloneFlags,
    sys::stat::Mode,
};
use oci_spec::runtime::{LinuxNamespace, LinuxNamespaceType};

/// `set_namespace` moves the container process into namespaces associated with different paths.
/// For more information, see the [setns(2)](https://man7.org/linux/man-pages/man2/setns.2.html)
/// man page.
pub fn set_namespace(namespace_list: &Vec<LinuxNamespace>) -> Result<()> {
    for namespace in namespace_list {
        if let Some(path) = namespace.path() {
            let fd = fcntl::open(path.as_os_str(), OFlag::empty(), Mode::empty()).context(
                format!("failed to open the namespace file: {}", path.display()),
            )?;
            sched::setns(fd.as_raw_fd(), linux_namespace_to_clone_flags(namespace)).context(
                format!("failed to enter the namespace file: {}", path.display()),
            )?;
        }
    }
    Ok(())
}

/// `linux_namespace_to_clone_flags` converts a [LinuxNamespace] to [CloneFlags].
/// For more information, see the [clone(2)](https://man7.org/linux/man-pages/man2/clone.2.html)
/// man page.
pub fn linux_namespace_to_clone_flags(namespace: &LinuxNamespace) -> CloneFlags {
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
