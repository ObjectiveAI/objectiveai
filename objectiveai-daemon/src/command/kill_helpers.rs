//! Shared kill logic for the `{mcp,viewer} kill` commands, `update`'s
//! pre-install teardown, and the `{api,db} config` mutation handlers'
//! kill-on-config-change ([`kill_api_before_config_change`] /
//! [`kill_api_after_config_change`] and the gate-held db pair
//! [`kill_db_before_config_change`] /
//! [`kill_db_after_config_change`]). (The api / db / laboratories
//! kill commands and `kill-all` were retired — `daemon kill` is the
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
/// wait → hard kill. Fallible core: `Ok(count)` (0 or 1) when the
/// child is gone — a missing/already-dead child is `Ok(0)`, never an
/// error — and `Err` only when a LIVE child could not be terminated
/// (its exit wait failed, or the hard kill after the grace window
/// errored).
pub async fn try_kill_resident_child(
    global: &GlobalContext,
    key: &str,
) -> Result<usize, Error> {
    let Some(mut child) = global.take_resident_child(key) else {
        return Ok(0);
    };
    let Some(pid) = child.id() else {
        // Already reaped.
        return Ok(0);
    };
    // Graceful first: Unix SIGTERM (handlers run — the laboratory
    // host stops its containers), Windows TerminateProcess.
    let _ = objectiveai_sdk::process::kill_pid(pid);
    match tokio::time::timeout(TERM_GRACE, child.wait()).await {
        Ok(Ok(_status)) => Ok(1),
        Ok(Err(e)) => Err(Error::Spawn(format!("wait for killed {key} child"), e)),
        Err(_) => {
            // Didn't exit in the grace window — hard kill (and reap).
            child
                .kill()
                .await
                .map_err(|e| Error::Spawn(format!("hard-kill {key} child"), e))?;
            Ok(1)
        }
    }
}

/// Best-effort form of [`try_kill_resident_child`]: a kill failure
/// still counts the child as terminated (it was taken off the map
/// either way — the old behavior of the kill commands).
pub async fn kill_resident_child(global: &GlobalContext, key: &str) -> usize {
    try_kill_resident_child(global, key).await.unwrap_or(1)
}

/// How long a stdin-EOF graceful shutdown gets before the hard kill.
/// Generous: the laboratory host stops/evaporates every container it
/// serves on the way out, and podman (its machine VM included) can be
/// slow.
const GRACEFUL_STDIN_EOF_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// GRACEFUL kill of the laboratory-host resident `key` child: take it
/// off the map — which drops the [`LabHostStdio`] and closes the
/// host's stdin — and let the resulting EOF drive its own shutdown
/// (`server.stop_started`: regular containers stopped, ephemerals
/// evaporated). Unlike [`try_kill_resident_child`] this sends NO signal
/// first: on Windows `TerminateProcess` is a hard, ungraceful kill that
/// would race the EOF, so we WAIT for the host to exit on its own and
/// only hard-kill as a fallback if it overruns the grace window.
///
/// `Ok(0)` when nothing was running (never an error); `Ok(1)` once the
/// child is gone (graceful exit or fallback hard kill); `Err` only when
/// a live child could neither be waited on nor hard-killed.
pub async fn graceful_kill_resident_child(
    global: &GlobalContext,
    key: &str,
) -> Result<usize, Error> {
    let Some(mut child) = global.take_resident_child(key) else {
        return Ok(0);
    };
    if child.id().is_none() {
        // Already reaped.
        return Ok(0);
    }
    // stdin is now closed (the map held the only lasting `LabHostStdio`
    // Arc). Wait for the host's own EOF-driven graceful shutdown.
    match tokio::time::timeout(GRACEFUL_STDIN_EOF_GRACE, child.wait()).await {
        Ok(Ok(_status)) => Ok(1),
        Ok(Err(e)) => Err(Error::Spawn(format!("wait for graceful {key} child"), e)),
        Err(_) => {
            // Overran the grace window — hard kill (and reap).
            child
                .kill()
                .await
                .map_err(|e| Error::Spawn(format!("hard-kill {key} child"), e))?;
            Ok(1)
        }
    }
}

/// Retire the resident db BEFORE a `db config` mutation is written —
/// under [`GlobalContext::db_init_gate`], which is what makes the kill
/// airtight: every db rebuild (including the whole spawn it may
/// perform) runs under that gate, so holding it here guarantees no
/// child is mid-birth and no handle-store can race the kill. The
/// cached [`crate::db::DbHandle`] is invalidated in the same critical
/// section (the respawned local db binds a NEW random port, so the old
/// pool can never be reused). FALLIBLE — a live db that cannot be
/// terminated aborts the config change. Not running is `Ok`. May wait
/// out an in-flight cold db spawn (seconds) — a rare admin op paying
/// for correctness.
pub async fn kill_db_before_config_change(
    global: &GlobalContext,
) -> Result<(), Error> {
    let gate = global.db_init_gate();
    let _guard = gate.lock().await;
    try_kill_resident_child(global, "db").await?;
    global.invalidate_db().await;
    Ok(())
}

/// The AFTER-write sweep paired with
/// [`kill_db_before_config_change`]: a rebuild starting between the
/// first kill and the write landing resolved the OLD config — waiting
/// on the gate here serializes after it, then retires its child and
/// clears its stored handle, so the next acquire rebuilds on the NEW
/// config. Best-effort by design: the write already landed.
pub async fn kill_db_after_config_change(global: &GlobalContext) {
    let gate = global.db_init_gate();
    let _guard = gate.lock().await;
    let _ = kill_resident_child(global, "db").await;
    global.invalidate_db().await;
}

/// Retire the resident api server BEFORE an `api config` mutation is
/// written: the running server was spawned with (and its address
/// resolved under) the config being replaced. FALLIBLE — a live api
/// that cannot be terminated aborts the config change, so a stale
/// server never survives a set. Not running is `Ok` (nothing to
/// retire).
pub async fn kill_api_before_config_change(
    global: &GlobalContext,
) -> Result<(), Error> {
    try_kill_resident_child(global, "api").await.map(|_| ())
}

/// The AFTER-write sweep paired with
/// [`kill_api_before_config_change`]: a concurrent request may have
/// respawned the api against the OLD config in the window between the
/// first kill and the write landing — retire that straggler too.
/// Best-effort by design (the write already landed; the change is in
/// effect for every later spawn): its failure is ignored.
pub async fn kill_api_after_config_change(global: &GlobalContext) {
    let _ = kill_resident_child(global, "api").await;
}

/// The viewer's respawn half of a `daemon config` mutation
/// (`daemon config set`, `refresh-secret-signature-pair`): the
/// viewer's whole daemon-facing config (`DAEMON_ADDRESS` /
/// `DAEMON_SIGNATURE`) is frozen into its env at spawn, so a config
/// change can only reach a RUNNING viewer through a fresh spawn.
/// `viewer_was_running` is the caller's BEFORE-the-write sample of
/// [`GlobalContext::server_child_alive`]`("viewer")` — only a viewer
/// the user already had up gets bounced; a mutation never turns into
/// a surprise viewer launch. The kill is best-effort (an unkillable
/// viewer stays up on the old env and the spawn below reuses it); the
/// respawn is FATAL — the write already landed, but the caller should
/// hear that their viewer did not come back.
pub async fn respawn_viewer_after_config_change(
    global: &GlobalContext,
    scoped: &crate::context::ScopedContext,
    viewer_was_running: bool,
) -> Result<(), Error> {
    if !viewer_was_running {
        return Ok(());
    }
    let _ = kill_resident_child(global, "viewer").await;
    crate::command::viewer::spawn::spawn(global, scoped).await.map(|_| ())
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
