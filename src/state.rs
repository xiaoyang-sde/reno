use procfs::process::ProcState;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{error::RuntimeError, process::inspect_process};

const OCI_VERSION: &str = "1.0.2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Creating,
    Created,
    Running,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub oci_version: String,
    pub id: String,
    pub bundle: PathBuf,
    pub status: Status,
    pub pid: i32,
    pub annotations: Option<HashMap<String, String>>,
}

impl State {
    pub fn new(id: String, bundle: PathBuf) -> Self {
        State {
            oci_version: String::from(OCI_VERSION),
            id,
            bundle,
            status: Status::Creating,
            pid: -1,
            annotations: Some(HashMap::new()),
        }
    }

    pub fn load(container_path: &Path) -> Result<Self, RuntimeError> {
        let state_file_path = &container_path.join("state.json");
        let state_json = read_to_string(state_file_path).map_err(|err| RuntimeError {
            message: format!(
                "failed to read the state from {}: {}",
                state_file_path.display(),
                err
            ),
        })?;

        let state: State = serde_json::from_str(&state_json).map_err(|err| RuntimeError {
            message: format!("failed to deserialize the state from JSON: {}", err),
        })?;

        Ok(state)
    }

    pub fn persist(&self, container_path: &Path) -> Result<(), RuntimeError> {
        let state_json = serde_json::to_string(&self).map_err(|err| RuntimeError {
            message: format!("failed to serialize the state to JSON: {}", err),
        })?;

        let state_file_path = &container_path.join("state.json");
        let mut state_file = File::create(state_file_path).map_err(|err| RuntimeError {
            message: format!("failed to create {}: {}", state_file_path.display(), err),
        })?;

        state_file
            .write_all(state_json.as_bytes())
            .map_err(|err| RuntimeError {
                message: format!(
                    "failed to write the state to {}: {}",
                    state_file_path.display(),
                    err
                ),
            })?;

        Ok(())
    }

    pub fn refresh(&mut self) {
        if self.pid == -1 {
            return;
        }

        if let Ok(state) = inspect_process(self.pid) {
            match state {
                ProcState::Running | ProcState::Sleeping | ProcState::Waiting => {
                    self.status = Status::Running;
                }
                ProcState::Tracing | ProcState::Stopped | ProcState::Zombie | ProcState::Dead => {
                    self.status = Status::Stopped;
                }
                _ => (),
            }
        } else {
            self.status = Status::Stopped;
        }
    }
}
