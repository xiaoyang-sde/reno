use anyhow::Result;
use nix::unistd;

/// `set_hostname` updates the system hostname to the given string.
/// For more information, see the [sethostname(2)](https://man7.org/linux/man-pages/man2/gethostname.2.html)
/// man page.
pub fn set_hostname(hostname: &String) -> Result<()> {
    unistd::sethostname(hostname)?;
    Ok(())
}
