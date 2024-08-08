use anyhow::{Context, Result};
use nix::sys::resource::{setrlimit, Resource};
use oci_spec::runtime::{PosixRlimit, PosixRlimitType};

/// `set_rlimit` sets a soft and hard limit for each resource.
/// The soft limit is the value that the kernel enforces for the resource.
/// The hard limit is a maximum value for the soft limit.
/// For example, `RLIMIT_CPU` limits the amount of CPU time the container process could consume.
/// For more information, see the [setrlimit(2)](https://man7.org/linux/man-pages/man2/setrlimit.2.html)
/// man page.
pub fn set_rlimit(rlimit: &PosixRlimit) -> Result<()> {
    let resource = posix_rlimit_type_to_resource(&rlimit.typ());
    setrlimit(resource, rlimit.soft(), rlimit.hard())
        .context(format!("failed to set resource limit for {}", rlimit.typ()))?;
    Ok(())
}

/// `posix_rlimit_type_to_resource` converts [PosixRlimitType] to [Resource].
fn posix_rlimit_type_to_resource(rlimit: &PosixRlimitType) -> Resource {
    match rlimit {
        PosixRlimitType::RlimitCpu => Resource::RLIMIT_CPU,
        PosixRlimitType::RlimitFsize => Resource::RLIMIT_FSIZE,
        PosixRlimitType::RlimitData => Resource::RLIMIT_DATA,
        PosixRlimitType::RlimitStack => Resource::RLIMIT_STACK,
        PosixRlimitType::RlimitCore => Resource::RLIMIT_CORE,
        PosixRlimitType::RlimitRss => Resource::RLIMIT_RSS,
        PosixRlimitType::RlimitNproc => Resource::RLIMIT_NPROC,
        PosixRlimitType::RlimitNofile => Resource::RLIMIT_NOFILE,
        PosixRlimitType::RlimitMemlock => Resource::RLIMIT_MEMLOCK,
        PosixRlimitType::RlimitAs => Resource::RLIMIT_AS,
        PosixRlimitType::RlimitLocks => Resource::RLIMIT_LOCKS,
        PosixRlimitType::RlimitSigpending => Resource::RLIMIT_SIGPENDING,
        PosixRlimitType::RlimitMsgqueue => Resource::RLIMIT_MSGQUEUE,
        PosixRlimitType::RlimitNice => Resource::RLIMIT_NICE,
        PosixRlimitType::RlimitRtprio => Resource::RLIMIT_RTPRIO,
        PosixRlimitType::RlimitRttime => Resource::RLIMIT_RTTIME,
    }
}
