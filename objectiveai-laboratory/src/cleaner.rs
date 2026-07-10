//! The orphan-container cleaner: stop RUNNING laboratory containers
//! whose managers are gone (hard-killed — graceful shutdowns stop
//! their own containers).
//!
//! One [`sweep`] runs per manager, spawned right after its first
//! successful daemon connection. The protocol is race-free against
//! every concurrent connect and every other cleaner:
//!
//! - per candidate id, the cleaner `try_acquire`s the BARE-id GUARD —
//!   the same guard `connect` wait_acquires around its connection-lock
//!   acquisition. A held guard means a connect is mid-acquisition (or
//!   another cleaner owns the id): skip, the next sweep reconsiders.
//! - holding the guard, connection locks are probed with the read-only
//!   [`objectiveai_sdk::lockfile::try_held`] — deliberately NOT
//!   `try_acquire`: a transient probe claim would be visible to
//!   `laboratories connect`'s `spawn_until_lock_published` pre-probe
//!   (`try_read`), which would report "already connected" without a
//!   manager existing. `try_held` takes nothing and publishes nothing.
//! - a manager past the guard always holds its connection lock (the
//!   guard is released only after acquisition), so the probe sees it;
//!   one not yet past the guard blocks until the sweep finishes, then
//!   `start` revives whatever was stopped.
//!
//! Crash safety: every lock is kernel-released on process death, so a
//! cleaner killed at ANY instant leaks nothing; the only side effect
//! is an idempotent `podman stop`, reversed by any later connect.
//!
//! Parallelism is two-level: every laboratory id is its own async call
//! (all joined concurrently), and within one id every per-address
//! liveness probe is joined concurrently too.

use std::path::PathBuf;

use objectiveai_sdk::client_objectiveai_mcp::laboratory::parse_connect_lock_key;

use crate::podman;

/// One full sweep over this state's RUNNING laboratory containers.
/// Errors are reported to stderr and never propagate — cleaning is
/// best-effort by design.
pub async fn sweep(bin_dir: PathBuf, state: String, lock_dir: PathBuf) {
    let podman = podman::Podman::new(bin_dir);
    let ids = match podman::laboratory::list_running(&podman, &state).await {
        Ok(ids) => ids,
        Err(e) => {
            eprintln!("cleaner: list running laboratories: {e}");
            return;
        }
    };
    // Level 1: every laboratory id concurrently.
    futures::future::join_all(
        ids.into_iter()
            .map(|id| clean_id(&podman, &state, &lock_dir, id)),
    )
    .await;
}

/// Evaluate one laboratory id and stop its container if no manager
/// anywhere holds a connection lock for it.
async fn clean_id(
    podman: &podman::Podman,
    state: &str,
    lock_dir: &PathBuf,
    id: String,
) {
    // The guard: excludes concurrent connects (and other cleaners) for
    // exactly the check+stop window. Held elsewhere → not our turn.
    let Some(guard) = objectiveai_sdk::lockfile::try_acquire(
        lock_dir,
        &id,
        &format!("cleaner pid {}", std::process::id()),
    )
    .await
    else {
        return;
    };

    // Under the guard: enumerate this id's connection-lock keys (rule
    // B — trailing `.` + exactly 22 base62 chars) and probe them all
    // concurrently (level 2). Read-only probes; see the module docs.
    let keys = match objectiveai_sdk::lockfile::keys_in_dir(lock_dir).await {
        Ok(keys) => keys,
        Err(e) => {
            eprintln!("cleaner: enumerate locks: {e}");
            let _ = guard.release();
            return;
        }
    };
    let connection_keys: Vec<String> = keys
        .into_iter()
        .filter(|key| matches!(parse_connect_lock_key(key), Some((key_id, _)) if key_id == id))
        .collect();
    let held = futures::future::join_all(
        connection_keys
            .iter()
            .map(|key| objectiveai_sdk::lockfile::try_held(lock_dir, key)),
    )
    .await;

    if !held.into_iter().any(|held| held) {
        // No live manager for this id anywhere — stop (never remove)
        // its container. Failure just leaves it for the next sweep.
        if let Err(e) = podman::laboratory::stop(podman, state, &id).await {
            eprintln!("cleaner: stop laboratory '{id}': {e}");
        }
    }
    if let Err(e) = guard.release() {
        eprintln!("cleaner: release guard '{id}': {e}");
    }
}
