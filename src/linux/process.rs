use anyhow::{Context, Result};
use nix::{
    sched::{self, CloneFlags},
    unistd::Pid,
};
use oci_spec::runtime::LinuxNamespace;
use procfs::process::{ProcState, Process};

use crate::linux::namespace;

/// `clone_child` creates a child process that invokes `function` in seperated
/// Linux namespaces specified in `namespace_list`.
/// For more information, see the [clone(2)](https://man7.org/linux/man-pages/man2/clone.2.html)
/// man page.
pub fn clone_child(
    child_fn: impl FnMut() -> isize,
    namespace_list: &[LinuxNamespace],
) -> Result<Pid> {
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let mut stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

    let clone_flags = namespace_list
        .iter()
        .map(namespace::linux_namespace_to_clone_flags)
        .reduce(|flag_1, flag_2| flag_1 | flag_2)
        .unwrap_or(CloneFlags::empty());

    let pid = sched::clone(Box::new(child_fn), &mut stack, clone_flags, None)
        .context("failed to clone the container process")?;
    Ok(pid)
}

/// `inspect_process` inspects the status of the process in `/proc/<pid>/stat`
/// and returns a variant of the [ProcState] enum that represents the process status.
pub fn inspect_process(pid: i32) -> Result<ProcState> {
    let process = Process::new(pid).context(format!("failed to inspect the process {}", pid))?;
    let process_stat = process
        .stat()
        .context(format!("failed to inspect the status of process {}", pid))?;
    let state = process_stat
        .state()
        .context(format!("failed to inspect the state of process {}", pid))?;
    Ok(state)
}
