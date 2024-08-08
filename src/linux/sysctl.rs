use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};

/// `set_sysctl` modifies kernel parameters for the container.
/// The parameters are listed under `/proc/sys/`, such as
/// `net/ipv4/tcp_congestion_control`.
/// For more information, see the [sysctl(8)](https://man7.org/linux/man-pages/man8/sysctl.8.html)
/// man page.
pub fn set_sysctl(kernel_parameter_map: &HashMap<String, String>) -> Result<()> {
    for (parameter, value) in kernel_parameter_map {
        let path = &Path::new("/proc/sys").join(parameter.replace('.', "/"));
        fs::write(path, value).context(format!(
            "failed to write {} to {}",
            value,
            path.display()
        ))?;
    }
    Ok(())
}

/// `set_oom_score_adj` sets the `oom_score_adj` for the container process.
/// The `oom_score_adj` is an integer between `-1000` to `1000`.
/// The lower the value, the lower the chance that it's going to be killed by the Out of Memory killer.
pub fn set_oom_score_adj(oom_score_adj: i32) -> Result<()> {
    let sysctl_path = Path::new("/proc/self/oom_score_adj");
    fs::write(sysctl_path, oom_score_adj.to_string())
        .context(format!("failed to set oom_score_adj to {}", oom_score_adj))?;
    Ok(())
}
