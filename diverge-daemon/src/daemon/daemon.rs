//! The daemon's pieces, built once.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::Live;
use crate::database::Target;
use crate::judge::DaemonAuthenticator;
use crate::providers::ProviderBook;
use crate::store::Store;

/// The daemon: its records, what is live, and where the logs are.
/// Built once at start and shared behind an `Arc` by every connection
/// and every scope.
#[derive(Debug)]
pub struct Daemon {
    /// The records.
    pub store: Store,
    /// The daemon's side of every handshake: its own tokens, and whom
    /// it admits.
    pub authenticator: Arc<DaemonAuthenticator>,
    /// The providers it works with: their records, as they stand.
    pub providers: Arc<ProviderBook>,
    /// What is nobody's record.
    pub live: Live,
    /// `<dir>`, under which every log is — `agents/`, `tools/` and
    /// `dependency_tools/`: see [`logs`](crate::logs).
    pub logs: PathBuf,
    /// How long a container may go unused before its run is ended:
    /// `idle_seconds` of the configuration.
    pub idle: Duration,
    /// The database the daemon serves, as the daemon reaches it: see
    /// [`database`](crate::database).
    pub database: Target,
    /// The daemon's stop: sent `true` by a loopback `POST /stop` (see
    /// [`stop`](crate::serve::stop)), or by the signal, and read by
    /// the listener, which drains on it. Made by [`run`](crate::serve::run)
    /// and held here so a handler can reach it.
    pub stop: watch::Sender<bool>,
}

impl Daemon {
    /// A daemon on an open store, admitting by `authenticator`,
    /// working with the providers whose records `providers` holds,
    /// holding its agents' logs under `logs`, ending a run unused for `idle`, serving the database
    /// `database`, stopped through `stop`, with nothing live yet.
    pub fn new(store: Store, authenticator: Arc<DaemonAuthenticator>, providers: Arc<ProviderBook>, logs: PathBuf, idle: Duration, database: Target, stop: watch::Sender<bool>) -> Self {
        Daemon {
            store,
            authenticator,
            providers,
            live: Live::new(),
            logs,
            idle,
            database,
            stop,
        }
    }
}
