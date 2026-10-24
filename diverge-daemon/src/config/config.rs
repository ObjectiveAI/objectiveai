//! The document itself.

use std::collections::BTreeMap;

use diverge_sdk::daemon::DEFAULT_PORT;
use serde::{Deserialize, Serialize};

use super::{Daemons, Postgres, Providers};

/// The whole of `daemon.yaml`: what the daemon is told.
///
/// An absent file is this type's [`Default`]. An unknown key is an
/// error, so a misspelled setting is refused rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The TCP port the daemon accepts the protocol on: a WebSocket
    /// upgrade at any path, on every interface. Absent means
    /// [`DEFAULT_PORT`], one above the port a provider listens on, so
    /// that a daemon and a provider on one host do not contend for it.
    pub port: u16,
    /// The daemon's own identities: for each issuer, the subject whose
    /// token it presents there, in every handshake from that issuer —
    /// with its clients, its providers and other daemons alike. An
    /// issuer absent here is one the daemon trusts no token from, since
    /// it could not answer from it: a client, a provider and a remote
    /// daemon all present a token from an issuer listed here. Each is
    /// an identity of the root's
    /// [`identities`](diverge_sdk::identities), signed in beforehand.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub identities: BTreeMap<String, String>,
    /// The providers the daemon works with: see [`Providers`]. Absent,
    /// none.
    pub providers: Providers,
    /// The other daemons this daemon works with through its providers:
    /// see [`Daemons`]. Absent, none.
    pub daemons: Daemons,
    /// The database the daemon keeps its records in and serves to its
    /// containers: see [`Postgres`]. Read at start and never changed
    /// while the daemon runs: another database is another start.
    /// Absent means local, with its defaults.
    pub postgres: Postgres,
    /// How many seconds a container — an agent or a tool — may go
    /// unused before the daemon ends its run. The clock resets on
    /// every use and does not run while the container is active; the
    /// record stays, and the next use starts the container again.
    /// Absent means ten.
    pub idle_seconds: u64,
    /// Whether the daemon accepts connections from other daemons
    /// through the providers it is connected to: when `true`, it opens
    /// the provider protocol's `daemons::accept` on every provider
    /// connection it holds, and admits each daemon that connects as it
    /// admits any client: by its identity, which must be an account's. When `false`,
    /// it opens none, and no daemon reaches it or the tools it serves.
    /// Absent means `true`.
    pub accept_daemons: bool,
    /// How many seconds the daemon waits, once it has cached a
    /// connected provider's answer to its `capacity` ask, before it
    /// asks again. The wait starts when the answer is cached, so a
    /// slow provider stretches it; an act of the daemon's own that
    /// changes the answer — a run started or ended, a volume made,
    /// resized or dropped, a FUSE serve opened or closed — asks at
    /// once, and nothing waits for the answer. Absent means ten.
    pub capacity_poll_seconds: u64,
}

/// What a daemon runs on before it has written a line of
/// configuration: the port one above the provider's, no identity, no
/// provider and no remote daemon, a Postgres of
/// its own, ten seconds of idleness, other daemons accepted, and
/// every provider's capacity asked every ten seconds.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: DEFAULT_PORT,
            identities: BTreeMap::new(),
            providers: Providers::default(),
            daemons: Daemons::default(),
            postgres: Postgres::default(),
            idle_seconds: 10,
            accept_daemons: true,
            capacity_poll_seconds: 10,
        }
    }
}
