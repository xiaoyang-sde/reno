use anyhow::Result;
use oci_spec::runtime::{LinuxNamespace, Spec};

use crate::{
    hook,
    linux::{device, hostname, mount, namespace, sysctl},
    state::State,
};

pub fn init_environment(
    spec: &Spec,
    state: &State,
    namespace_list: &[LinuxNamespace],
) -> Result<()> {
    namespace::set_namespace(namespace_list)?;

    let rootfs = &state.bundle.join(spec.root().as_ref().unwrap().path());
    mount::mount_rootfs(rootfs)?;

    if let Some(mounts) = &spec.mounts() {
        for mount in mounts {
            mount::custom_mount(rootfs, mount)?;
        }
    }

    if let Some(linux) = spec.linux() {
        if let Some(devices) = linux.devices() {
            for device in devices {
                device::create_device(rootfs, device)?;
            }
        }
    }

    device::create_default_device(rootfs)?;
    device::create_default_symlink(rootfs)?;

    if let Some(hostname) = spec.hostname() {
        hostname::set_hostname(hostname)?;
    }

    Ok(())
}

pub fn create_container(spec: &Spec, state: &State) -> Result<()> {
    if let Some(hooks) = spec.hooks() {
        if let Some(create_container_hooks) = hooks.create_container() {
            for create_container_hook in create_container_hooks {
                hook::run_hook(state, create_container_hook)?;
            }
        }
    }

    let rootfs = state.bundle.join(spec.root().as_ref().unwrap().path());
    let readonly = spec.root().as_ref().unwrap().readonly().unwrap_or_default();
    mount::pivot_rootfs(&rootfs, readonly)?;

    if let Some(linux) = spec.linux() {
        if let Some(sysctl) = linux.sysctl() {
            sysctl::set_sysctl(sysctl)?;
        }
    }
    Ok(())
}
