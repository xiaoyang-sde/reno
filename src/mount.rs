use nix::mount::umount2;
use nix::mount::{mount, MntFlags, MsFlags};
use nix::unistd::chdir;
use nix::unistd::pivot_root;
use oci_spec::runtime::Mount;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::Path;

use crate::error::RuntimeError;

/// `mount_rootfs` changes the propagation type of the root mount
/// from shared to private, and then remounts the root mount to
/// clone it in the current namespace
pub fn mount_rootfs(rootfs: &Path) -> Result<(), RuntimeError> {
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    )
    .map_err(|err| RuntimeError {
        message: format!("failed to mount the rootfs: {}", err),
    })?;

    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )
    .map_err(|err| RuntimeError {
        message: format!("failed to mount the rootfs: {}", err),
    })?;

    Ok(())
}

/// `pivot_rootfs` changes the root mount in the mount namespace.
pub fn pivot_rootfs(rootfs: &Path) -> Result<(), RuntimeError> {
    chdir(rootfs).map_err(|err| RuntimeError {
        message: format!("failed to run chdir: {}", err),
    })?;

    create_dir_all(rootfs.join("root_archive")).map_err(|err| RuntimeError {
        message: format!("failed to create ./root_archive: {}", err),
    })?;

    // `pivot_root` moves the root mount to `root_archive` and makes `rootfs` as the new root mount
    pivot_root(rootfs.as_os_str(), rootfs.join("root_archive").as_os_str()).map_err(|err| {
        RuntimeError {
            message: format!("failed to run pivot_root {}: {}", rootfs.display(), err),
        }
    })?;

    umount2("./root_archive", MntFlags::MNT_DETACH).map_err(|err| RuntimeError {
        message: format!("failed to unmount ./root_archive: {}", err),
    })?;

    remove_dir_all("./root_archive").map_err(|err| RuntimeError {
        message: format!("failed to remove ./root_archive: {}", err),
    })?;

    chdir("/").map_err(|err| RuntimeError {
        message: format!("failed to run chdir: {}", err),
    })?;
    Ok(())
}

/// `oci_mount` accepts a `mount` struct defined in the bundle configuration
/// and mounts the source to the destination with specified options
pub fn oci_mount(rootfs: &Path, m: &Mount) -> Result<(), RuntimeError> {
    let destination = rootfs.join(
        m.destination()
            .display()
            .to_string()
            .trim_start_matches('/'),
    );
    if !destination.exists() {
        create_dir_all(&destination).map_err(|err| RuntimeError {
            message: format!("failed to create {}: {}", destination.display(), err),
        })?;
    }

    let mount_flags = {
        if m.typ() == &Some(String::from("bind")) {
            MsFlags::MS_BIND
        } else {
            MsFlags::empty()
        }
    };

    mount(
        m.source().as_ref(),
        &destination,
        m.typ().as_deref(),
        mount_flags,
        None::<&str>,
    )
    .map_err(|err| RuntimeError {
        message: format!("failed to mount to {}: {}", destination.display(), err),
    })?;

    Ok(())
}
