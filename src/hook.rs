use oci_spec::runtime::Hook;
use std::{
    io::Write,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::{error::RuntimeError, state::State};

/// `run_hook` accepts and invokes a [Hook], which is a command that is run at a particular event
/// in the lifecycle of a container.
pub fn run_hook(state: &State, hook: &Hook) -> Result<(), RuntimeError> {
    let mut command = Command::new(hook.path());
    command.env_clear();

    if let Some(env_list) = hook.env() {
        for env in env_list {
            if let Some((k, v)) = env.split_once('=') {
                command.env(k, v);
            }
        }
    }

    if let Some(args) = hook.args() {
        command.arg0(&args[0]);
        command.args(&args[1..]);
    }

    let mut hook_process = command.stdin(Stdio::piped()).spawn()?;

    if let Some(mut stdin) = hook_process.stdin.as_ref() {
        let state_json = serde_json::to_string(state).map_err(|err| RuntimeError {
            message: format!("failed to serialize the state to JSON: {}", err),
        })?;
        stdin.write_all(state_json.as_bytes())?;
    }

    let status = hook_process.wait()?;
    if let Some(code) = status.code() {
        if code == 0 {
            Ok(())
        } else {
            Err(RuntimeError {
                message: format!("failed to run the hook: exit status {}", code),
            })
        }
    } else {
        Err(RuntimeError {
            message: "failed to run the hook".to_string(),
        })
    }
}
