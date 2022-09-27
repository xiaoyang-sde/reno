use nix::mount::umount2;
use nix::mount::{mount, MntFlags, MsFlags};
use nix::unistd::chdir;
use nix::unistd::pivot_root;
use oci_spec::runtime::Mount;
use std::fs::{create_dir_all, remove_dir_all};
use std::{error::Error, path::Path};

/// `mount_rootfs` changes the propagation type of the root mount
/// from shared to private, and then remounts the root mount to remove it
/// from the original file system.
pub fn mount_rootfs(rootfs: &Path) -> Result<(), Box<dyn Error>> {
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    )?;

    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    Ok(())
}

/// `pivot_rootfs` changes the root mount in the mount namespace.
pub fn pivot_rootfs(rootfs: &Path) -> Result<(), Box<dyn Error>> {
    chdir(rootfs)?;
    create_dir_all(rootfs.join("root_archive"))?;
    // Move the root mount to `root_archive`
    pivot_root(rootfs.as_os_str(), rootfs.join("root_archive").as_os_str())?;

    umount2("./root_archive", MntFlags::MNT_DETACH)?;
    remove_dir_all("./root_archive")?;
    chdir("/")?;
    Ok(())
}

/// `oci_mount` accepts a `mount` struct defined in the bundle configuration
/// and mounts the source to the destination with specified options
pub fn oci_mount(rootfs: &Path, m: &Mount) -> Result<(), Box<dyn Error>> {
    let destination = rootfs.join(m.destination());
    if !destination.exists() {
        create_dir_all(&destination)?;
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
    )?;

    Ok(())
}
