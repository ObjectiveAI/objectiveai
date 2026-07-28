//! Shared kill logic for the `{mcp,viewer,laboratories} kill`
//! commands, `update`'s pre-install teardown, and the `{api,db}`
//! config mutation handlers' kill-on-config-change
//! ([`kill_api_before_config_change`] /
//! [`kill_api_after_config_change`] and the gate-held db pair
//! [`kill_db_before_config_change`] /
//! [`kill_db_after_config_change`]). `daemon kill` is the
//! whole-teardown path: killing the daemon takes every leashed
//! resident child with it.
//!
//! A server is one of the daemon's LEASHED resident children
//! (metadata on [`crate::context::GlobalContext`]; the
//! [`tokio::process::Child`] itself is owned by its spawn's lifecycle
//! task — there are no server lockfiles to resolve pids through
//! anymore, and no legacy lock-owner sweep either). Killing one means
//! removing its map entry (generation-guarded) and driving the
//! child-appropriate shutdown via its lifecycle task: the stdio child
//! (laboratory host) dies GRACEFULLY by the stdin EOF the removal
//! itself causes; everything else gets SIGTERM (the viewer tears down
//! its windows; Windows gets `TerminateProcess`, its only option), a
//! bounded wait, then SIGKILL. See [`kill_resident_child`]. For db,
//! killing the supervisor takes the postmaster with it (job object /
//! `PR_SET_PDEATHSIG`).
//!
//! Scope is inherently THIS daemon: other states' servers belong to
//! other daemons and die with them.

use crate::context::GlobalContext;
use crate::error::Error;

/// How long the signal path's SIGTERM gets before the SIGKILL
/// escalation. (Signal-killed children only — the stdio child's
/// graceful EOF shutdown is unbounded by design.)
const TERM_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// THE kill for a resident `key` child — kill semantics are declared
/// by the child's own shape, not chosen per call site:
///
/// - **stdio child** (the laboratory host — it holds a
///   [`crate::context::LabHostStdio`]): GRACEFUL, always. Removing the
///   map entry drops the stdio Arc, closing the host's stdin — EOF is
///   its shutdown signal (`server.stop_started`: regular containers
///   stopped, ephemerals evaporated). NO signal first (on Windows
///   `TerminateProcess` is a hard, ungraceful kill that would race the
///   EOF), NO hard-kill fallback, UNBOUNDED wait: `stop_started` is
///   itself bounded by `podman stop` (SIGTERM→SIGKILL on podman's own
///   grace), and a cold podman machine legitimately takes minutes. A
///   genuinely wedged host holds this future open; the operator falls
///   back to killing the daemon (the OS leash takes the host with it).
/// - **everything else** (db / api / mcp / viewer): signal path —
///   `Term` (SIGTERM; handlers run / `TerminateProcess`), a bounded
///   [`TERM_GRACE`] wait, then `Kill` (SIGKILL) and an unbounded wait
///   (SIGKILL always lands; the exit is only an OS-reap away).
///
/// Signals are routed THROUGH the child's lifecycle task
/// ([`crate::context::KillRequest`]) — it owns the un-reaped `Child`,
/// so a signal can never hit a recycled pid — and the exit is awaited
/// on the death watch the same task fires from `child.wait()`.
/// INFALLIBLE by construction: a closed kill channel or death watch
/// means the child already exited. Returns the number of children
/// terminated (0 or 1); an absent/already-dead child is 0, never an
/// error.
///
/// The entry is removed UP FRONT (generation-guarded, so a racing
/// respawn's successor is never touched): concurrent spawns see the
/// key as absent immediately — a dying old-config server can never be
/// "reused" during its own teardown, which is what keeps the api/db
/// config brackets airtight.
pub async fn kill_resident_child(global: &GlobalContext, key: &str) -> usize {
    let Some(snapshot) = global.resident_child_kill_snapshot(key) else {
        return 0;
    };
    let mut dead_rx = snapshot.dead_rx;
    // Remove OUR entry first — for the stdio child this IS the kill
    // signal (stdin EOF once in-flight `lab_host_stdio` borrowers drop
    // their clones — the host is alive and acking, so that's bounded).
    global.remove_resident_child_if(key, snapshot.generation);
    if *dead_rx.borrow() {
        // Already exited (the removal above was just cleanup).
        return 0;
    }
    if snapshot.has_stdio {
        // Graceful EOF shutdown — wait, unbounded, for true exit. A
        // closed watch = the lifecycle task is gone = the child
        // exited.
        let _ = dead_rx.changed().await;
        return 1;
    }
    // Signal path. Send errors mean the lifecycle task already
    // observed the exit — nothing left to kill.
    if snapshot
        .kill_tx
        .send(crate::context::KillRequest::Term)
        .is_err()
    {
        return 1;
    }
    if tokio::time::timeout(TERM_GRACE, dead_rx.changed())
        .await
        .is_err()
    {
        // Didn't exit in the grace window — escalate to SIGKILL and
        // wait it out (unbounded: SIGKILL always lands).
        let _ = snapshot.kill_tx.send(crate::context::KillRequest::Kill);
        let _ = dead_rx.changed().await;
    }
    1
}

/// Retire the resident db BEFORE a `db config` mutation is written —
/// under [`GlobalContext::db_init_gate`], which is what makes the kill
/// airtight: every db rebuild (including the whole spawn it may
/// perform) runs under that gate, so holding it here guarantees no
/// child is mid-birth and no handle-store can race the kill. The
/// cached [`crate::db::DbHandle`] is invalidated in the same critical
/// section (the respawned local db binds a NEW random port, so the old
/// pool can never be reused). The kill always lands (signal path
/// escalates to SIGKILL) — a stale server never survives a set. Not
/// running is a no-op. May wait out an in-flight cold db spawn
/// (seconds) — a rare admin op paying for correctness.
pub async fn kill_db_before_config_change(
    global: &GlobalContext,
) -> Result<(), Error> {
    let gate = global.db_init_gate();
    let _guard = gate.lock().await;
    kill_resident_child(global, "db").await;
    global.invalidate_db().await;
    Ok(())
}

/// The AFTER-write sweep paired with
/// [`kill_db_before_config_change`]: a rebuild starting between the
/// first kill and the write landing resolved the OLD config — waiting
/// on the gate here serializes after it, then retires its child and
/// clears its stored handle, so the next acquire rebuilds on the NEW
/// config. The write already landed either way.
pub async fn kill_db_after_config_change(global: &GlobalContext) {
    let gate = global.db_init_gate();
    let _guard = gate.lock().await;
    kill_resident_child(global, "db").await;
    global.invalidate_db().await;
}

/// Retire the resident api server BEFORE an `api config` mutation is
/// written: the running server was spawned with (and its address
/// resolved under) the config being replaced. The kill always lands
/// (signal path escalates to SIGKILL), so a stale server never
/// survives a set. Not running is a no-op. `Result` kept for caller
/// symmetry with the db bracket.
pub async fn kill_api_before_config_change(
    global: &GlobalContext,
) -> Result<(), Error> {
    kill_resident_child(global, "api").await;
    Ok(())
}

/// The AFTER-write sweep paired with
/// [`kill_api_before_config_change`]: a concurrent request may have
/// respawned the api against the OLD config in the window between the
/// first kill and the write landing — retire that straggler too. The
/// write already landed; the change is in effect for every later
/// spawn.
pub async fn kill_api_after_config_change(global: &GlobalContext) {
    kill_resident_child(global, "api").await;
}

/// Bounce a RUNNING viewer so it picks up daemon-side state that is
/// frozen at spawn.
///
/// The viewer's whole daemon-facing input is frozen at spawn: its env
/// (`DAEMON_ADDRESS` / `DAEMON_SIGNATURE`, mutated by `daemon config
/// set` and `refresh-secret-signature-pair`) and its argv (the
/// development-plugin registrations, mutated by `development plugins
/// viewer create`/`delete`). A change to either reaches a running
/// viewer only through a fresh spawn — deliberately: one propagation
/// mechanism, no live channel to keep consistent.
///
/// `viewer_was_running` is the caller's BEFORE-the-mutation sample of
/// [`GlobalContext::server_child_alive`]`("viewer")` — only a viewer
/// the user already had up gets bounced; a mutation never turns into
/// a surprise viewer launch (an absent viewer picks the new state up
/// whenever it is next spawned, since spawn reads it fresh). The kill
/// is best-effort (an unkillable viewer stays up on the old state and
/// the spawn below reuses it); the respawn is FATAL — the mutation
/// already landed, but the caller should hear that their viewer did
/// not come back.
pub async fn respawn_running_viewer(
    global: &GlobalContext,
    scoped: &crate::context::ScopedContext,
    viewer_was_running: bool,
) -> Result<(), Error> {
    if !viewer_was_running {
        return Ok(());
    }
    kill_resident_child(global, "viewer").await;
    crate::command::viewer::spawn::spawn(global, scoped).await.map(|_| ())
}

