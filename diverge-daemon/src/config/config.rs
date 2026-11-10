//! The document itself.

use diverge_sdk::daemon::DEFAULT_PORT;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;
use serde::{Deserialize, Serialize};

use super::Postgres;

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
    /// Who the daemon is: the one identity whose token it presents on
    /// every connection it accepts — to a client, a provider and
    /// another daemon alike, whatever issuer theirs is from — and to
    /// every provider it dials. It is an identity of the root's
    /// [`identities`](diverge_sdk::identities), signed in beforehand.
    /// It lists no issuer the daemon trusts: whom the daemon admits is
    /// its accounts', its administrators' and its providers' records'
    /// to say, from whatever issuer each is. Absent, the daemon has no
    /// identity, and completes no handshake.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    /// The identities served as administrators: each admitted as a
    /// client whether or not it is an account, holding every grant
    /// there is. What makes a fresh daemon, which has no account,
    /// reachable at all: the first account is made by an administrator.
    /// Absent, none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub admins: Vec<Identity>,
    /// What the daemon says of itself to every provider it is
    /// listable to, which lists it beside the daemon's identity:
    /// one line of plain text, of at most 132 characters — see
    /// [`Description`]. It rides the listable flag, so a provider the
    /// daemon is not listable to is not told it. Absent, none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
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
/// description, no provider and no remote daemon, a Postgres of
/// its own, ten seconds of idleness, other daemons accepted, and
/// every provider's capacity asked every ten seconds.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: DEFAULT_PORT,
            identity: None,
            admins: Vec::new(),
            description: None,
            postgres: Postgres::default(),
            idle_seconds: 10,
            accept_daemons: true,
            capacity_poll_seconds: 10,
        }
    }
}
