use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeError {
    pub message: String,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

impl Error for RuntimeError {}
