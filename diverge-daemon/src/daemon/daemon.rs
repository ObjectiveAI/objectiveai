//! The daemon's pieces, built once.

use super::Live;
use crate::store::Store;

/// The daemon: its records, and what is live. Built once at start and
/// shared behind an `Arc` by every connection and every scope.
#[derive(Debug)]
pub struct Daemon {
    /// The records.
    pub store: Store,
    /// What is nobody's record.
    pub live: Live,
}

impl Daemon {
    /// A daemon on an open store, with nothing live yet.
    pub fn new(store: Store) -> Self {
        Daemon {
            store,
            live: Live::new(),
        }
    }
}
