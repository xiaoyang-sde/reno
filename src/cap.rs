use caps::{all, drop, has_cap, CapSet, Capability as CapsCap};
use oci_spec::runtime::Capability as OCICap;

use crate::error::RuntimeError;

pub fn set_cap(cap_set: CapSet, cap: &OCICap) -> Result<(), RuntimeError> {
    let cap = oci_spec_to_caps(cap);
    match cap_set {
        CapSet::Bounding => {
            if all().contains(&cap) {
                drop(None, CapSet::Bounding, cap).map_err(|err| RuntimeError {
                    message: format!("failed to drop {}: {}", cap, err),
                })?;
            }
        }
        _ => {
            let has_cap = has_cap(None, cap_set, cap).map_err(|err| RuntimeError {
                message: format!("failed to check if {} exists: {}", cap, err),
            })?;
            if !has_cap {
                caps::raise(None, cap_set, cap).map_err(|err| RuntimeError {
                    message: format!("failed to raise {}: {}", cap, err),
                })?;
            }
        }
    }

    Ok(())
}

fn oci_spec_to_caps(cap: &OCICap) -> CapsCap {
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
