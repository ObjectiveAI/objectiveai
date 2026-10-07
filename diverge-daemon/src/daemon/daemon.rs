//! The daemon's pieces, built once.

use std::path::PathBuf;
use std::time::Duration;

use super::Live;
use crate::store::Store;

/// The daemon: its records, what is live, and where the content is.
/// Built once at start and shared behind an `Arc` by every connection
/// and every scope.
#[derive(Debug)]
pub struct Daemon {
    /// The records.
    pub store: Store,
    /// What is nobody's record.
    pub live: Live,
    /// `<dir>/resources/`, where every resource's bytes are: see
    /// [`content`](crate::content).
    pub resources: PathBuf,
    /// `<dir>/agents/`, where every agent's log is: see
    /// [`logs`](crate::logs).
    pub logs: PathBuf,
    /// `<dir>/overlays/`, where an ephemeral mount's own layer is for
    /// the run's life: see [`fuse`](crate::containers::fuse).
    pub overlays: PathBuf,
    /// How long a container may go unused before its run is ended:
    /// `idle_seconds` of the configuration.
    pub idle: Duration,
}

impl Daemon {
    /// A daemon on an open store, holding its content under
    /// `resources`, its agents' logs under `logs` and its runs'
    /// overlays under `overlays`, ending a run unused for `idle`, with
    /// nothing live yet.
    pub fn new(store: Store, resources: PathBuf, logs: PathBuf, overlays: PathBuf, idle: Duration) -> Self {
        Daemon {
            store,
            live: Live::new(),
            resources,
            logs,
            overlays,
            idle,
        }
    }

    /// Where an upload still arriving is kept.
    pub fn incoming(&self) -> PathBuf {
        self.resources.join("incoming")
    }
}
