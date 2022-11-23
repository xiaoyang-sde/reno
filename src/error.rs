use caps::errors::CapsError;
use nix::Error as NixError;
use oci_spec::OciSpecError;
use procfs::ProcError;
use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
    io::Error as IOError,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    pub fn new(message: String) -> Self {
        RuntimeError { message }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

impl Error for RuntimeError {}

impl From<IOError> for RuntimeError {
    fn from(err: IOError) -> Self {
        RuntimeError {
            message: err.to_string(),
        }
    }
}

impl From<NixError> for RuntimeError {
    fn from(err: NixError) -> Self {
        RuntimeError {
            message: err.to_string(),
        }
    }
}

impl From<OciSpecError> for RuntimeError {
    fn from(err: OciSpecError) -> Self {
        RuntimeError {
            message: err.to_string(),
        }
    }
}

impl From<ProcError> for RuntimeError {
    fn from(err: ProcError) -> Self {
        RuntimeError {
            message: err.to_string(),
        }
    }
}

impl From<CapsError> for RuntimeError {
    fn from(err: CapsError) -> Self {
        RuntimeError {
            message: err.to_string(),
        }
    }
}
