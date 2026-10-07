//! The daemon's pieces, built once.

use std::path::PathBuf;

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
}

impl Daemon {
    /// A daemon on an open store, holding its content under
    /// `resources` and its agents' logs under `logs`, with nothing
    /// live yet.
    pub fn new(store: Store, resources: PathBuf, logs: PathBuf) -> Self {
        Daemon {
            store,
            live: Live::new(),
            resources,
            logs,
        }
    }

    /// Where an upload still arriving is kept.
    pub fn incoming(&self) -> PathBuf {
        self.resources.join("incoming")
    }
}
