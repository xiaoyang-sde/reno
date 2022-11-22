use std::fs::write;
use std::{collections::HashMap, path::Path};

use crate::error::RuntimeError;

/// `set_sysctl` modifies kernel parameters for the container.
/// For more information, see the [sysctl(8)](https://man7.org/linux/man-pages/man8/sysctl.8.html)
/// man page.
pub fn set_sysctl(sysctl: &HashMap<String, String>) -> Result<(), RuntimeError> {
    for (field, value) in sysctl {
        let sysctl_path = Path::new("/proc/sys").join(field.replace('.', "/"));
        write(sysctl_path, value).map_err(|err| RuntimeError::new(err.to_string()))?;
    }
    Ok(())
}
