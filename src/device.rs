use nix::sys::stat::SFlag;
use nix::sys::stat::{makedev, mknod, Mode};

use nix::unistd::{chown, Gid, Uid};
use oci_spec::runtime::{LinuxDevice, LinuxDeviceBuilder, LinuxDeviceType};

use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use crate::error::RuntimeError;

/// `create_default_symlink` creates symbolic links for the default
/// [dev symbolic links](https://github.com/opencontainers/runtime-spec/blob/main/runtime-linux.md#-dev-symbolic-links)
/// specified in OCI runtime specification
pub fn create_default_symlink(rootfs: &Path) -> Result<(), RuntimeError> {
    let default_symlink_list = [
        ("/proc/self/fd", "/dev/fd"),
        ("/proc/self/fd/0", "/dev/stdin"),
        ("/proc/self/fd/1", "/dev/stdout"),
        ("/proc/self/fd/2", "/dev/stderr"),
    ];

    for (source, destination) in default_symlink_list {
        symlink(source, rootfs.join(destination.trim_start_matches('/'))).map_err(|err| {
            RuntimeError {
                message: format!("failed to create default symlink: {}", err),
            }
        })?;
    }
    Ok(())
}

fn to_sflag(flag: LinuxDeviceType) -> SFlag {
    match flag {
        LinuxDeviceType::C | LinuxDeviceType::U => SFlag::S_IFCHR,
        LinuxDeviceType::B => SFlag::S_IFBLK,
        LinuxDeviceType::P => SFlag::S_IFIFO,
        _ => SFlag::empty(),
    }
}

pub fn create_device(rootfs: &Path, device: &LinuxDevice) -> Result<(), RuntimeError> {
    let path = &rootfs.join(device.path().display().to_string().trim_start_matches('/'));

    mknod(
        path,
        to_sflag(device.typ()),
        Mode::from_bits_truncate(device.file_mode().unwrap_or(0o066)),
        makedev(device.major() as u64, device.minor() as u64),
    )
    .map_err(|err| RuntimeError {
        message: format!(
            "failed to create the device {}: {}",
            device.path().display(),
            err
        ),
    })?;

    if let Some(uid) = device.uid() {
        chown(path, Some(Uid::from_raw(uid)), None).map_err(|err| RuntimeError {
            message: format!("failed to create default symlink: {}", err.desc()),
        })?;
    }
    if let Some(gid) = device.gid() {
        chown(path, None, Some(Gid::from_raw(gid))).map_err(|err| RuntimeError {
            message: format!("failed to create default symlink: {}", err.desc()),
        })?;
    }

    Ok(())
}

/// `create_default_device` creates devices for the
/// [default devices](https://github.com/opencontainers/runtime-spec/blob/main/config-linux.md#default-devices)
/// specified in OCI runtime specification
pub fn create_default_device(rootfs: &Path) {
    let default_device_list: [(&str, LinuxDeviceType, u32, u32, u32, u32, u32); 7] = [
        ("/dev/null", LinuxDeviceType::C, 1, 3, 0o066, 0, 0),
        ("/dev/zero", LinuxDeviceType::C, 1, 5, 0o066, 0, 0),
        ("/dev/full", LinuxDeviceType::C, 1, 7, 0o066, 0, 0),
        ("/dev/random", LinuxDeviceType::C, 1, 8, 0o066, 0, 0),
        ("/dev/urandom", LinuxDeviceType::C, 1, 9, 0o066, 0, 0),
        ("/dev/tty", LinuxDeviceType::C, 5, 0, 0o066, 0, 0),
        ("/dev/ptmx", LinuxDeviceType::C, 5, 2, 0o066, 0, 0),
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
            .build()
            .unwrap();

        create_device(rootfs, &device).unwrap();
    }
}
