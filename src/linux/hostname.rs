use crate::error::RuntimeError;

use nix::unistd::sethostname;

/// `set_hostname` updates the system hostname to the given string.
/// For more information, see the [sethostname(2)](https://man7.org/linux/man-pages/man2/gethostname.2.html)
/// man page.
pub fn set_hostname(hostname: &String) -> Result<(), RuntimeError> {
    sethostname(hostname).map_err(|err| RuntimeError::new(err.to_string()))?;
    Ok(())
}
