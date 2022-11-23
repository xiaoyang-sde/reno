use std::collections::HashSet;

use anyhow::{Context, Result};
use caps::{self, CapSet, Capability as CapsCap};
use oci_spec::runtime::{Capabilities, Capability as OCICap};

/// `set_cap` sets Linux capabilities for the container process.
/// It drops extra capabilities for the bounding set, and raises capabilities for other sets.
/// For more information, see the [capabilities(7)](https://man7.org/linux/man-pages/man7/capabilities.7.html)
/// man page.
pub fn set_cap(cap_set: CapSet, capabilities: &Capabilities) -> Result<()> {
    let capabilities: &HashSet<CapsCap> = &capabilities.iter().map(oci_cap_to_caps_cap).collect();
    match cap_set {
        CapSet::Bounding => {
            let existing_capabilities = caps::read(None, CapSet::Bounding)
                .context("failed to read the bounding capabilities")?;
            for cap in existing_capabilities.difference(capabilities) {
                caps::drop(None, CapSet::Bounding, *cap).context(format!(
                    "failed to drop {} from the bounding capabilities",
                    cap
                ))?;
            }
        }
        _ => {
            caps::set(None, cap_set, capabilities).context("failed to set the capabilities")?;
        }
    }
    Ok(())
}

/// `oci_cap_to_caps_cap` converts [OCICap] to [CapsCap].
fn oci_cap_to_caps_cap(cap: &OCICap) -> CapsCap {
    match cap {
        OCICap::AuditControl => CapsCap::CAP_AUDIT_CONTROL,
        OCICap::AuditRead => CapsCap::CAP_AUDIT_READ,
        OCICap::AuditWrite => CapsCap::CAP_AUDIT_WRITE,
        OCICap::BlockSuspend => CapsCap::CAP_BLOCK_SUSPEND,
        OCICap::Bpf => CapsCap::CAP_BPF,
        OCICap::CheckpointRestore => CapsCap::CAP_CHECKPOINT_RESTORE,
        OCICap::Chown => CapsCap::CAP_CHOWN,
        OCICap::DacOverride => CapsCap::CAP_DAC_OVERRIDE,
        OCICap::DacReadSearch => CapsCap::CAP_DAC_READ_SEARCH,
        OCICap::Fowner => CapsCap::CAP_FOWNER,
        OCICap::Fsetid => CapsCap::CAP_FSETID,
        OCICap::IpcLock => CapsCap::CAP_IPC_LOCK,
        OCICap::IpcOwner => CapsCap::CAP_IPC_OWNER,
        OCICap::Kill => CapsCap::CAP_KILL,
        OCICap::Lease => CapsCap::CAP_LEASE,
        OCICap::LinuxImmutable => CapsCap::CAP_LINUX_IMMUTABLE,
        OCICap::MacAdmin => CapsCap::CAP_MAC_ADMIN,
        OCICap::MacOverride => CapsCap::CAP_MAC_OVERRIDE,
        OCICap::Mknod => CapsCap::CAP_MKNOD,
        OCICap::NetAdmin => CapsCap::CAP_NET_ADMIN,
        OCICap::NetBindService => CapsCap::CAP_NET_BIND_SERVICE,
        OCICap::NetBroadcast => CapsCap::CAP_NET_BROADCAST,
        OCICap::NetRaw => CapsCap::CAP_NET_RAW,
        OCICap::Perfmon => CapsCap::CAP_PERFMON,
        OCICap::Setgid => CapsCap::CAP_SETGID,
        OCICap::Setfcap => CapsCap::CAP_SETFCAP,
        OCICap::Setpcap => CapsCap::CAP_SETPCAP,
        OCICap::Setuid => CapsCap::CAP_SETUID,
        OCICap::SysAdmin => CapsCap::CAP_SYS_ADMIN,
        OCICap::SysBoot => CapsCap::CAP_SYS_BOOT,
        OCICap::SysChroot => CapsCap::CAP_SYS_CHROOT,
        OCICap::SysModule => CapsCap::CAP_SYS_MODULE,
        OCICap::SysNice => CapsCap::CAP_SYS_NICE,
        OCICap::SysPacct => CapsCap::CAP_SYS_PACCT,
        OCICap::SysPtrace => CapsCap::CAP_SYS_PTRACE,
        OCICap::SysRawio => CapsCap::CAP_SYS_RAWIO,
        OCICap::SysResource => CapsCap::CAP_SYS_RESOURCE,
        OCICap::SysTime => CapsCap::CAP_SYS_TIME,
        OCICap::SysTtyConfig => CapsCap::CAP_SYS_TTY_CONFIG,
        OCICap::Syslog => CapsCap::CAP_SYSLOG,
        OCICap::WakeAlarm => CapsCap::CAP_WAKE_ALARM,
    }
}
