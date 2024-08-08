use anyhow::{Context, Result};
use nix::unistd;

/// `set_hostname` updates the system hostname to the given string.
/// For more information, see the [sethostname(2)](https://man7.org/linux/man-pages/man2/gethostname.2.html)
/// man page.
pub fn set_hostname(hostname: &str) -> Result<()> {
    unistd::sethostname(hostname).context("failed to set the system hostname")?;
    Ok(())
}
