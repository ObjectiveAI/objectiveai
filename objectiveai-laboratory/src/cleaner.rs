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

/// One full sweep over this state's RUNNING laboratory containers,
/// plus the leftover plugin-build checkouts under `<bin>/temp` (a
/// hard-killed predecessor's scratch — new builds mint fresh uuid
/// dirs, and nothing builds until a channel serves, so nothing races
/// this). Errors are reported to stderr and never propagate —
/// cleaning is best-effort by design.
pub async fn sweep(bin_dir: PathBuf, state: String) {
    crate::gitrepo::sweep_temp(&bin_dir).await;
    let podman = podman::Podman::new(bin_dir);
    let ids = match podman::laboratory::list_running(&podman, &state).await {
        Ok(ids) => ids,
        Err(e) => {
            eprintln!("cleaner: list running laboratories: {e}");
            return;
        }
    };
    // Every stop concurrently — they are independent containers.
    let results = futures::future::join_all(
        ids.iter()
            .map(|id| podman::laboratory::stop(&podman, &state, id)),
    )
    .await;
    for (id, result) in ids.iter().zip(results) {
        if let Err(e) = result {
            eprintln!("cleaner: stop laboratory '{id}': {e}");
        }
    }
}
