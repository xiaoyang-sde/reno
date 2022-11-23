use std::fs;
use std::{collections::HashMap, path::Path};

use crate::error::RuntimeError;

/// `set_sysctl` modifies kernel parameters for the container.
/// The parameters are listed under `/proc/sys/`, such as
/// `net/ipv4/tcp_congestion_control`.
/// For more information, see the [sysctl(8)](https://man7.org/linux/man-pages/man8/sysctl.8.html)
/// man page.
pub fn set_sysctl(kernel_parameter_map: &HashMap<String, String>) -> Result<(), RuntimeError> {
    for (parameter, value) in kernel_parameter_map {
        let path = Path::new("/proc/sys").join(parameter.replace('.', "/"));
        fs::write(path, value)?;
    }
    Ok(())
}
