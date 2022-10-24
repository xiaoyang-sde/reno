use nix::mount::umount2;
use nix::mount::{mount, MntFlags, MsFlags};
use nix::unistd::chdir;
use nix::unistd::pivot_root;
use oci_spec::runtime::Mount;
use std::ffi::OsString;
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

fn options_to_mount_flags(m: &Mount) -> (MsFlags, OsString) {
    let mut mount_flags = MsFlags::empty();
    let mut mount_data = Vec::new();

    if let Some(options) = &m.options() {
        for option in options {
            if let Some((is_clear, flag)) = match option.as_str() {
                "defaults" => Some((false, MsFlags::empty())),
                "ro" => Some((false, MsFlags::MS_RDONLY)),
                "rw" => Some((true, MsFlags::MS_RDONLY)),
                "suid" => Some((true, MsFlags::MS_NOSUID)),
                "nosuid" => Some((false, MsFlags::MS_NOSUID)),
                "dev" => Some((true, MsFlags::MS_NODEV)),
                "nodev" => Some((false, MsFlags::MS_NODEV)),
                "exec" => Some((true, MsFlags::MS_NOEXEC)),
                "noexec" => Some((false, MsFlags::MS_NOEXEC)),
                "sync" => Some((false, MsFlags::MS_SYNCHRONOUS)),
                "async" => Some((true, MsFlags::MS_SYNCHRONOUS)),
                "dirsync" => Some((false, MsFlags::MS_DIRSYNC)),
                "remount" => Some((false, MsFlags::MS_REMOUNT)),
                "mand" => Some((false, MsFlags::MS_MANDLOCK)),
                "nomand" => Some((true, MsFlags::MS_MANDLOCK)),
                "atime" => Some((true, MsFlags::MS_NOATIME)),
                "noatime" => Some((false, MsFlags::MS_NOATIME)),
                "diratime" => Some((true, MsFlags::MS_NODIRATIME)),
                "nodiratime" => Some((false, MsFlags::MS_NODIRATIME)),
                "bind" => Some((false, MsFlags::MS_BIND)),
                "rbind" => Some((false, MsFlags::MS_BIND | MsFlags::MS_REC)),
                "unbindable" => Some((false, MsFlags::MS_UNBINDABLE)),
                "runbindable" => Some((false, MsFlags::MS_UNBINDABLE | MsFlags::MS_REC)),
                "private" => Some((true, MsFlags::MS_PRIVATE)),
                "rprivate" => Some((true, MsFlags::MS_PRIVATE | MsFlags::MS_REC)),
                "shared" => Some((true, MsFlags::MS_SHARED)),
                "rshared" => Some((true, MsFlags::MS_SHARED | MsFlags::MS_REC)),
                "slave" => Some((true, MsFlags::MS_SLAVE)),
                "rslave" => Some((true, MsFlags::MS_SLAVE | MsFlags::MS_REC)),
                "relatime" => Some((true, MsFlags::MS_RELATIME)),
                "norelatime" => Some((true, MsFlags::MS_RELATIME)),
                "strictatime" => Some((true, MsFlags::MS_STRICTATIME)),
                "nostrictatime" => Some((true, MsFlags::MS_STRICTATIME)),
                _ => None,
            } {
                if is_clear {
                    mount_flags &= !flag;
                } else {
                    mount_flags |= flag;
                }
            } else {
                mount_data.push(option.as_str());
            }
        }
    }

    (mount_flags, mount_data.join(",").into())
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

    let (mount_flags, mount_data) = options_to_mount_flags(m);

    mount(
        m.source().as_ref(),
        &destination,
        m.typ().as_deref(),
        mount_flags,
        Some(mount_data).as_deref(),
    )
    .map_err(|err| RuntimeError {
        message: format!("failed to mount to {}: {}", destination.display(), err),
    })?;

    Ok(())
}
