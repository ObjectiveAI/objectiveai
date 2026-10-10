//! The daemon's pieces, built once.

use std::path::PathBuf;
use std::time::Duration;

use super::Live;
use crate::database::Target;
use crate::store::Store;

/// The daemon: its records, what is live, and where the logs are.
/// Built once at start and shared behind an `Arc` by every connection
/// and every scope.
#[derive(Debug)]
pub struct Daemon {
    /// The records.
    pub store: Store,
    /// What is nobody's record.
    pub live: Live,
    /// `<dir>/agents/`, where every agent's log is: see
    /// [`logs`](crate::logs).
    pub logs: PathBuf,
    /// How long a container may go unused before its run is ended:
    /// `idle_seconds` of the configuration.
    pub idle: Duration,
    /// The database the daemon serves, as the daemon reaches it: see
    /// [`database`](crate::database).
    pub database: Target,
    /// Whether the daemon accepts connections from other daemons
    /// through every provider it is connected to: `accept_daemons` of
    /// the configuration.
    pub accept_daemons: bool,
}

impl Daemon {
    /// A daemon on an open store, holding its agents' logs under
    /// `logs`, ending a run unused for `idle`, serving the database
    /// `database`, accepting other daemons when `accept_daemons`, with
    /// nothing live yet.
    pub fn new(store: Store, logs: PathBuf, idle: Duration, database: Target, accept_daemons: bool) -> Self {
        Daemon {
            store,
            live: Live::new(),
            logs,
            idle,
            database,
            accept_daemons,
        }
    }
}
