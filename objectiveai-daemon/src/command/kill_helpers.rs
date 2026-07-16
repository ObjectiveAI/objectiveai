//! Shared kill-by-lock-owner logic for the `{api,db,mcp,viewer} kill`
//! commands.
//!
//! A server's identity on disk is the lockfile it holds: the api at
//! `<dir>/bin/locks` key `api` (machine-wide, one lock), and db /
//! mcp / viewer at `<dir>/state/<state>/locks` keyed `db` / `mcp` /
//! `viewer` (per state). Killing a server means reading the live
//! owner PIDs of that lockfile via [`objectiveai_sdk::lockfile::owners`]
//! and terminating them — for db, killing the supervisor takes the
//! postmaster with it (job object / `PR_SET_PDEATHSIG`).

use std::path::PathBuf;

use crate::context::Context;
use crate::error::Error;

/// Read the owner PIDs of `(locks_dir, key)` and kill each. Returns
/// the count actually terminated. A lockfile with no live owner
/// yields zero — idempotent.
pub async fn kill_lock_owners(locks_dir: PathBuf, key: &str) -> Result<usize, Error> {
    let pids = objectiveai_sdk::lockfile::owners(&locks_dir, key)
        .await
        .map_err(|e| Error::Spawn(format!("read lock owners for {key}"), e))?;
    let mut killed = 0;
    for pid in pids {
        killed += crate::spawn::kill_pid(pid);
    }
    Ok(killed)
}

/// Kill the per-state lockfile owners for `key` across the scope:
/// `--state` hits only the current `OBJECTIVEAI_STATE`; `--global`
/// fans out across every `<dir>/state/<name>/locks` concurrently and
/// sums the kills.
pub async fn kill_per_state(
    ctx: &Context,
    scope: objectiveai_sdk::cli::command::SetScope,
    key: &'static str,
) -> Result<usize, Error> {
    match scope {
        objectiveai_sdk::cli::command::SetScope::State => {
            let locks_dir = ctx.filesystem.state_dir().join("locks");
            kill_lock_owners(locks_dir, key).await
        }
        objectiveai_sdk::cli::command::SetScope::Global => {
            let states_root = ctx.filesystem.dir().join("state");
            let mut dirs: Vec<PathBuf> = Vec::new();
            match tokio::fs::read_dir(&states_root).await {
                Ok(mut rd) => {
                    while let Some(entry) = rd
                        .next_entry()
                        .await
                        .map_err(|e| Error::Spawn("list states".to_string(), e))?
                    {
                        if entry
                            .file_type()
                            .await
                            .map(|t| t.is_dir())
                            .unwrap_or(false)
                        {
                            dirs.push(entry.path().join("locks"));
                        }
                    }
                }
                // No state dir at all → nothing to kill.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(Error::Spawn("open state dir".to_string(), e)),
            }
            let results = futures::future::join_all(
                dirs.into_iter().map(|d| kill_lock_owners(d, key)),
            )
            .await;
            let mut total = 0;
            for r in results {
                total += r?;
            }
            Ok(total)
        }
    }
}
