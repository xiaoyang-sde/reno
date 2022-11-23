use crate::error::RuntimeError;
use oci_spec::runtime::{LinuxRlimit, LinuxRlimitType};
use rlimit::Resource;

/// `set_rlimit` sets a soft and hard limit for each resource.
/// The soft limit is the value that the kernel enforces for the resource.
/// The hard limit is a maximum value for the soft limit.
/// For example, `RLIMIT_CPU` limits the amount of CPU time the container process could consume.
/// For more information, see the [getrlimit(2)](https://man7.org/linux/man-pages/man2/getrlimit.2.html)
/// man page.
pub fn set_rlimit(rlimit: &LinuxRlimit) -> Result<(), RuntimeError> {
    let resource = linux_rlimit_type_to_resource(&rlimit.typ());
    rlimit::setrlimit(resource, rlimit.soft(), rlimit.hard())?;
    Ok(())
}

/// `linux_rlimit_type_to_resource` converts [LinuxRlimitType] to [Resource].
fn linux_rlimit_type_to_resource(rlimit: &LinuxRlimitType) -> Resource {
    match rlimit {
        LinuxRlimitType::RlimitCpu => Resource::CPU,
        LinuxRlimitType::RlimitFsize => Resource::FSIZE,
        LinuxRlimitType::RlimitData => Resource::DATA,
        LinuxRlimitType::RlimitStack => Resource::STACK,
        LinuxRlimitType::RlimitCore => Resource::CORE,
        LinuxRlimitType::RlimitRss => Resource::RSS,
        LinuxRlimitType::RlimitNproc => Resource::NPROC,
        LinuxRlimitType::RlimitNofile => Resource::NOFILE,
        LinuxRlimitType::RlimitMemlock => Resource::MEMLOCK,
        LinuxRlimitType::RlimitAs => Resource::AS,
        LinuxRlimitType::RlimitLocks => Resource::LOCKS,
        LinuxRlimitType::RlimitSigpending => Resource::SIGPENDING,
        LinuxRlimitType::RlimitMsgqueue => Resource::MSGQUEUE,
        LinuxRlimitType::RlimitNice => Resource::NICE,
        LinuxRlimitType::RlimitRtprio => Resource::RTPRIO,
        LinuxRlimitType::RlimitRttime => Resource::RTTIME,
    }
}
