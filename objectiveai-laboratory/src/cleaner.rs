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
    let temp = bin_dir.join("temp");
    objectiveai_sdk::gitrepo::sweep_temp(&temp.join("daemon")).await;
    // The viewer-build partition: plugin checkouts and the staging
    // dirs their built assets are copied into — all uuid-named scratch
    // this host owns.
    objectiveai_sdk::gitrepo::sweep_temp(&temp.join("build")).await;
    // Migration: pre-split builds put checkouts directly under
    // `<bin>/temp` — clear those, matched by their UUID dir names so
    // the sibling partitions (`viewer`, `daemon-viewer`, whatever
    // comes next — each swept by its own process at ITS boot) are
    // never touched.
    if let Ok(mut entries) = tokio::fs::read_dir(&temp).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let is_uuid = name
                .to_str()
                .is_some_and(|name| uuid::Uuid::parse_str(name).is_ok());
            if is_uuid {
                objectiveai_sdk::gitrepo::remove_checkout(&entry.path()).await;
            }
        }
    }
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
