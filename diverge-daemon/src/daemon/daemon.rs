//! The daemon's pieces, built once.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::Live;
use crate::config::{Providers, Remote};
use crate::database::Target;
use crate::judge::DaemonAuthenticator;
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
    /// The providers it works with, as its configuration says.
    pub providers: Providers,
    /// The other daemons it reaches, as its configuration says.
    pub remotes: Vec<Remote>,
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
    /// How long a provider's capacity poller waits, once it has
    /// cached an answer, before it asks again: `capacity_poll_seconds`
    /// of the configuration.
    pub capacity_poll: Duration,
    /// The daemon's stop: sent `true` by a loopback `POST /stop` (see
    /// [`stop`](crate::serve::stop)), or by the signal, and read by
    /// the listener, which drains on it. Made by [`run`](crate::serve::run)
    /// and held here so a handler can reach it.
    pub stop: watch::Sender<bool>,
}

impl Daemon {
    /// A daemon on an open store, admitting by `authenticator`,
    /// working with the configuration's `providers` and `remotes`,
    /// holding its agents' logs under `logs`, ending a run unused for `idle`, serving the database
    /// `database`, accepting other daemons when `accept_daemons`,
    /// asking every provider's capacity again after `capacity_poll`,
    /// stopped through `stop`, with nothing live yet.
    pub fn new(store: Store, authenticator: Arc<DaemonAuthenticator>, providers: Providers, remotes: Vec<Remote>, logs: PathBuf, idle: Duration, database: Target, accept_daemons: bool, capacity_poll: Duration, stop: watch::Sender<bool>) -> Self {
        Daemon {
            store,
            authenticator,
            providers,
            remotes,
            live: Live::new(),
            logs,
            idle,
            database,
            accept_daemons,
            capacity_poll,
            stop,
        }
    }
}
