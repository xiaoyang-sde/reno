use nix::{
    sched::{clone, CloneFlags},
    unistd::Pid,
};
use oci_spec::runtime::{LinuxNamespace, LinuxNamespaceType};
use procfs::process::{ProcState, Process};

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

/// `clone_child` creates a child process that invokes `function` in seperated
/// Linux namespaces specified in `namespace_list`
pub fn clone_child(
    child_fn: impl FnMut() -> isize,
    namespaces: &[LinuxNamespace],
) -> Result<Pid, RuntimeError> {
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let mut stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

    let clone_flags = namespaces
        .iter()
        .map(namespace_to_clone_flag)
        .reduce(|flag_1, flag_2| flag_1 | flag_2)
        .unwrap_or(CloneFlags::empty());

    let pid =
        clone(Box::new(child_fn), &mut stack, clone_flags, None).map_err(|err| RuntimeError {
            message: format!("failed to invoke clone(): {}", err),
        })?;

    Ok(pid)
}

/// `inspect_process` inspects the status of the process in `/proc/<pid>/stat`
/// and returns a variant of the `ProcState` enum
pub fn inspect_process(pid: i32) -> Result<ProcState, RuntimeError> {
    let process = Process::new(pid).map_err(|err| RuntimeError {
        message: format!("failed to inspect the process {}: {}", pid, err),
    })?;

    let process_stat = process.stat().map_err(|err| RuntimeError {
        message: format!("failed to inspect the process status {}: {}", pid, err),
    })?;

    let state = process_stat.state().map_err(|err| RuntimeError {
        message: format!("failed to inspect the process state {}: {}", pid, err),
    })?;

    Ok(state)
}
