use nix::sys::stat::SFlag;
use nix::sys::stat::{self, Mode};

use nix::unistd::{self, Gid, Uid};
use oci_spec::runtime::{LinuxDevice, LinuxDeviceBuilder, LinuxDeviceType};

use anyhow::{Context, Result};
use std::fs::{self, Permissions};
use std::os::unix;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

/// `create_default_symlink` creates symbolic links for the default
/// [dev symbolic links](https://github.com/opencontainers/runtime-spec/blob/main/runtime-linux.md#-dev-symbolic-links)
/// specified in OCI runtime specification.
pub fn create_default_symlink(rootfs: &Path) -> Result<()> {
    let default_symlink_list = [
        ("/proc/self/fd", "/dev/fd"),
        ("/proc/self/fd/0", "/dev/stdin"),
        ("/proc/self/fd/1", "/dev/stdout"),
        ("/proc/self/fd/2", "/dev/stderr"),
        ("pts/ptmx", "/dev/ptmx"),
    ];

    for (source, destination) in default_symlink_list {
        unix::fs::symlink(source, rootfs.join(destination.trim_start_matches('/'))).context(
            format!(
                "failed to create default symlink from {} to {}",
                source, destination
            ),
        )?;
    }
    Ok(())
}

/// `linux_device_type_to_sflag` converts [LinuxDeviceType] to [SFlag].
fn linux_device_type_to_sflag(flag: LinuxDeviceType) -> SFlag {
    match flag {
        LinuxDeviceType::C | LinuxDeviceType::U => SFlag::S_IFCHR,
        LinuxDeviceType::B => SFlag::S_IFBLK,
        LinuxDeviceType::P => SFlag::S_IFIFO,
        _ => SFlag::empty(),
    }
}

/// `create_device` creates a Linux device with `mknod`.
/// For more information, see the [mknod(2)](https://man7.org/linux/man-pages/man2/mknod.2.html)
/// man page.
pub fn create_device(rootfs: &Path, device: &LinuxDevice) -> Result<()> {
    let path = &rootfs.join(device.path().display().to_string().trim_start_matches('/'));
    stat::mknod(
        path,
        linux_device_type_to_sflag(device.typ()),
        Mode::from_bits_truncate(device.file_mode().unwrap_or(0o066)),
        stat::makedev(device.major() as u64, device.minor() as u64),
    )
    .context(format!(
        "failed to create {} with mknod",
        device.path().display(),
    ))?;

    fs::set_permissions(path, Permissions::from_mode(0o660)).context(format!(
        "failed to change the permission of {}",
        path.display(),
    ))?;

    if let Some(gid) = device.gid() {
        unistd::chown(path, None, Some(Gid::from_raw(gid))).context(format!(
            "failed to create change the ownership of {} to group {}",
            device.path().display(),
            gid,
        ))?;
    }
    if let Some(uid) = device.uid() {
        unistd::chown(path, Some(Uid::from_raw(uid)), None).context(format!(
            "failed to create change the ownership of {} to user {}",
            device.path().display(),
            uid,
        ))?;
    }
    Ok(())
}

/// `create_default_device` creates devices for the
/// [default devices](https://github.com/opencontainers/runtime-spec/blob/main/config-linux.md#default-devices)
/// specified in OCI runtime specification.
pub fn create_default_device(rootfs: &Path) -> Result<()> {
    let default_device_list: [(&str, LinuxDeviceType, u32, u32, u32, u32, u32); 6] = [
        ("/dev/null", LinuxDeviceType::C, 1, 3, 0o066, 0, 0),
        ("/dev/zero", LinuxDeviceType::C, 1, 5, 0o066, 0, 0),
        ("/dev/full", LinuxDeviceType::C, 1, 7, 0o066, 0, 0),
        ("/dev/random", LinuxDeviceType::C, 1, 8, 0o066, 0, 0),
        ("/dev/urandom", LinuxDeviceType::C, 1, 9, 0o066, 0, 0),
        ("/dev/tty", LinuxDeviceType::C, 5, 0, 0o066, 0, 0),
    ];

    for (path, typ, major, minor, file_mode, uid, gid) in default_device_list {
        let device = LinuxDeviceBuilder::default()
            .path(PathBuf::from(path))
            .typ(typ)
            .major(major)
            .minor(minor)
            .file_mode(file_mode)
            .uid(uid)
            .gid(gid)
            .build()?;

        create_device(rootfs, &device)?;
    }
    Ok(())
}
