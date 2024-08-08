use crate::hook;
use crate::linux::cap;
use crate::linux::rlimit;
use crate::linux::sysctl;
use crate::state::State;
use anyhow::Context;
use caps::CapSet;

use anyhow::{bail, Result};
use nix::sys::prctl;
use nix::sys::stat;
use nix::sys::stat::Mode;
use nix::unistd;
use nix::unistd::Gid;
use nix::unistd::Uid;
use oci_spec::runtime::Spec;
use std::env;

pub fn start_container(spec: &Spec, state: &State) -> Result<()> {
    if let Some(hooks) = spec.hooks() {
        if let Some(start_container_hooks) = hooks.start_container() {
            for start_container_hook in start_container_hooks {
                hook::run_hook(state, start_container_hook)?;
            }
        }
    }

    if let Some(process) = spec.process() {
        if let Some(env_list) = process.env() {
            for env in env_list {
                if let Some((k, v)) = env.split_once('=') {
                    env::set_var(k, v);
                }
            }
        }

        if let Some(rlimits) = process.rlimits() {
            for rlimit in rlimits {
                rlimit::set_rlimit(rlimit)?;
            }
        }

        if let Some(oom_score_adj) = process.oom_score_adj() {
            sysctl::set_oom_score_adj(oom_score_adj)?;
        }

        if let Some(capabilities) = process.capabilities() {
            if let Some(capabilities) = capabilities.bounding() {
                cap::set_cap(CapSet::Bounding, capabilities)?;
            }
        }

        prctl::set_keepcaps(true).context("failed to set PR_SET_KEEPCAPS to true")?;
        unistd::setgid(Gid::from_raw(process.user().gid()))
            .context(format!("failed to set gid to {}", process.user().gid()))?;

        if let Some(mode) = process.user().umask() {
            if let Some(mode) = Mode::from_bits(mode) {
                stat::umask(mode);
            } else {
                bail!("invalid umask: {}", mode);
            }
        }

        if let Some(additional_gids) = process.user().additional_gids() {
            let additional_gids: &Vec<Gid> = &additional_gids
                .iter()
                .map(|gid| Gid::from_raw(*gid))
                .collect();
            unistd::setgroups(additional_gids)
                .context("failed to set additional gids".to_string())?;
        }
        unistd::setuid(Uid::from_raw(process.user().uid()))
            .context(format!("failed to set uid to {}", process.user().gid()))?;

        prctl::set_keepcaps(false).context("failed to set PR_SET_KEEPCAPS to false")?;

        if let Some(capabilities) = process.capabilities() {
            let capabilities_list = [
                (capabilities.effective(), CapSet::Effective),
                (capabilities.permitted(), CapSet::Permitted),
                (capabilities.inheritable(), CapSet::Inheritable),
                (capabilities.ambient(), CapSet::Ambient),
            ];
            for (capabilities, capabilities_set_flag) in capabilities_list.into_iter() {
                if let Some(capabilities) = capabilities {
                    if let Err(err) = cap::set_cap(capabilities_set_flag, capabilities) {
                        println!("{}", err);
                    }
                }
            }
        }

        unistd::chdir(process.cwd()).context(format!(
            "failed to change the working directory to {}",
            process.cwd().display()
        ))?;
    }
    Ok(())
}
