//! The orphan-container stopper: at host start, stop every RUNNING
//! laboratory container in this state.
//!
//! The host is the ONLY manager (it holds the single `laboratories`
//! lock for the state) and it starts containers strictly lazily, so at
//! this point anything already running was leaked by a hard-killed
//! predecessor (graceful shutdowns stop their own containers). No
//! per-id locks or liveness probes remain — the old per-(id, address)
//! manager protocol is gone.
//!
//! [`sweep`] runs synchronously BEFORE the daemon channels come up:
//! it must never race a lazily-started laboratory, and nothing can
//! start one until a channel is serving.
//!
//! Containers are STOPPED, never removed — an idempotent side effect
//! reversed by the next routed op's `start`.

use std::path::PathBuf;

use crate::podman;

/// One full sweep over this state's laboratory containers, plus the
/// leftover plugin-build checkouts under `<bin>/temp` (a hard-killed
/// predecessor's scratch — new builds mint fresh uuid dirs, and
/// nothing builds until a channel serves, so nothing races this).
///
/// Two partitions:
/// - EPHEMERAL leftovers (label carries a `response_id`) are REMOVED
///   — running or stopped: an ephemeral's lifetime was its single MCP
///   connection, which died with its host; whatever state the
///   container crashed in, it is garbage.
/// - REGULAR containers are STOPPED (running ones only), never
///   removed — their filesystems survive for the next lazy start.
///
/// Errors are reported to stderr and never propagate — cleaning is
/// best-effort by design.
pub async fn sweep(bin_dir: PathBuf, state: String) {
    crate::gitrepo::sweep_temp(&bin_dir).await;
    let podman = podman::Podman::new(bin_dir);
    let labs = match podman::laboratory::list(&podman, &state).await {
        Ok(labs) => labs,
        Err(e) => {
            eprintln!("cleaner: list laboratories: {e}");
            return;
        }
    };
    // Every action concurrently — they are independent containers.
    let actions: Vec<(String, bool)> = labs
        .into_iter()
        .filter_map(|lab| {
            if lab.response_id.is_some() {
                Some((lab.id, true))
            } else if lab.running {
                Some((lab.id, false))
            } else {
                None
            }
        })
        .collect();
    let results = futures::future::join_all(actions.iter().map(|(id, remove)| {
        let podman = &podman;
        let state = &state;
        async move {
            if *remove {
                podman::laboratory::remove(podman, state, id).await
            } else {
                podman::laboratory::stop(podman, state, id).await
            }
        }
    }))
    .await;
    for ((id, remove), result) in actions.iter().zip(results) {
        if let Err(e) = result {
            let verb = if *remove { "remove" } else { "stop" };
            eprintln!("cleaner: {verb} laboratory '{id}': {e}");
        }
    }
}
