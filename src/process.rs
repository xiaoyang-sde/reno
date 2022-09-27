use nix::{
    sched::{clone, CloneFlags},
    unistd::Pid,
};
use oci_spec::runtime::{LinuxNamespace, LinuxNamespaceType};

use crate::error::RuntimeError;

fn namespace_to_clone_flag(namespace: &LinuxNamespace) -> CloneFlags {
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

/// `clone_process` creates a child process that invokes `function` in seperated
/// Linux namespaces specified in `namespace_list`
pub fn clone_process(
    function: impl Fn() -> isize,
    namespace_list: &[LinuxNamespace],
) -> Result<Pid, RuntimeError> {
    let clone_flags = namespace_list
        .iter()
        .map(namespace_to_clone_flag)
        .reduce(|flag_1, flag_2| flag_1 | flag_2)
        .unwrap_or(CloneFlags::empty());

    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let stack: &mut [u8; STACK_SIZE] = &mut [0; STACK_SIZE];

    let pid = clone(Box::new(function), stack, clone_flags, None).map_err(|err| RuntimeError {
        message: format!("failed to clone(): {}", err.desc()),
    })?;

    Ok(pid)
}
