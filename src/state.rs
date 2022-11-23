use anyhow::{Context, Result};
use procfs::process::ProcState;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::linux::process::inspect_process;

const OCI_VERSION: &str = "1.0.2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Creating,
    Created,
    Running,
    Stopped,
}

/// The state of the container defined in the [runtime specification](https://github.com/opencontainers/runtime-spec/blob/main/runtime.md)
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

    /// `load` reads the container state from `{container_path}/state.json`.
    pub fn load(container_path: &Path) -> Result<Self> {
        let state_file_path = &container_path.join("state.json");
        let state_json = fs::read_to_string(state_file_path).context(format!(
            "failed to read the container state from {}",
            state_file_path.display()
        ))?;

        let state: State = serde_json::from_str(&state_json)
            .context("failed to deserialize the state from JSON".to_string())?;
        Ok(state)
    }

    /// `persist` serializes the container state to JSON and writes it to `{container_path}/state.json`.
    pub fn persist(&self, container_path: &Path) -> Result<()> {
        let state_json = serde_json::to_string(&self)
            .context("failed to serialize the state to JSON".to_string())?;

        let state_file_path = &container_path.join("state.json");
        let mut state_file = File::create(state_file_path).context(format!(
            "failed to create the container state file: {}",
            state_file_path.display()
        ))?;
        state_file
            .write_all(state_json.as_bytes())
            .context(format!(
                "failed to write container state to {}",
                state_file_path.display()
            ))?;
        Ok(())
    }

    /// `refresh` updates the container status based on the container process.
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

    /// `write_pid_file` writes the PID to `pid_file_path`.
    pub fn write_pid_file(&self, pid_file_path: &Path) -> Result<()> {
        let mut pid_file = File::create(pid_file_path).context(format!(
            "failed to create the PID file: {}",
            pid_file_path.display()
        ))?;
        pid_file
            .write_all(self.pid.to_string().as_bytes())
            .context(format!(
                "failed to write the PID to {}",
                pid_file_path.display()
            ))?;
        Ok(())
    }
}
