use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.message)
    }
}

impl Error for RuntimeError {}
