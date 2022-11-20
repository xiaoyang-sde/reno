use crate::error::RuntimeError;
use oci_spec::runtime::{LinuxRlimit, LinuxRlimitType};
use rlimit::{setrlimit, Resource};

pub fn set_rlimit(rlimit: &LinuxRlimit) -> Result<(), RuntimeError> {
    let resource = oci_spec_to_rlimit(&rlimit.typ());
    setrlimit(resource, rlimit.soft(), rlimit.hard()).map_err(|err| RuntimeError {
        message: format!("failed to set rlimit for {}: {}", resource.as_name(), err),
    })?;
    Ok(())
}

fn oci_spec_to_rlimit(rlimit: &LinuxRlimitType) -> Resource {
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
