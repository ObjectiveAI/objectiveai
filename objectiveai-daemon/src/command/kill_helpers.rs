//! Shared kill logic for the `{mcp,viewer} kill` commands and
//! `update`'s pre-install teardown. (The api / db / laboratories kill
//! commands and `kill-all` were retired — `daemon kill` is the
//! whole-teardown path: killing the daemon takes every leashed
//! resident child with it.)
//!
//! A server is one of the daemon's LEASHED resident children (held on
//! [`crate::context::GlobalContext`] since the stdout-readiness refactor — there are no
//! server lockfiles to resolve pids through anymore). Killing one
//! means taking its [`tokio::process::Child`] off the map and
//! terminating it: SIGTERM first (the laboratory host's handler stops
//! its containers; the viewer tears down its windows; Windows gets
//! `TerminateProcess`, its only option), a bounded wait, then a hard
//! kill. For db, killing the supervisor takes the postmaster with it
//! (job object / `PR_SET_PDEATHSIG`).
//!
//! Scope is inherently THIS daemon: other states' servers belong to
//! other daemons and die with them — the former cross-state lockfile
//! sweep (`kill_per_state`) is gone with the locks. The kill request
//! wire shapes keep their `scope` field for compatibility; both
//! scopes mean "this daemon's resident child".
//!
//! `kill_lock_owners` survives solely as `update`'s LEGACY sweep: an
//! in-place update over a ≤2.2.12 install may find old-style detached
//! servers still holding locks; killing them by owner pid is the only
//! way to reach them. Remove once updates from those versions stop
//! mattering.

use std::path::PathBuf;

use crate::context::GlobalContext;
use crate::error::Error;

/// How long the graceful SIGTERM gets before the hard kill.
const TERM_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// Kill this daemon's resident `key` child, if any: SIGTERM → bounded
/// wait → hard kill. Returns the count terminated (0 or 1) —
/// idempotent, a missing/already-dead child is a zero.
pub async fn kill_resident_child(global: &GlobalContext, key: &str) -> usize {
    let Some(mut child) = global.take_resident_child(key) else {
        return 0;
    };
    let Some(pid) = child.id() else {
        // Already reaped.
        return 0;
    };
    // Graceful first: Unix SIGTERM (handlers run — the laboratory
    // host stops its containers), Windows TerminateProcess.
    let _ = objectiveai_sdk::process::kill_pid(pid);
    match tokio::time::timeout(TERM_GRACE, child.wait()).await {
        Ok(_) => 1,
        Err(_) => {
            // Didn't exit in the grace window — hard kill (and reap).
            let _ = child.kill().await;
            1
        }
    }
}

/// LEGACY: read the owner PIDs of `(locks_dir, key)` and kill each —
/// the pre-2.2.13 servers held readiness locks; `update` sweeps them
/// so an in-place update can replace their binaries. Returns the
/// count actually terminated; no live owner yields zero.
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
