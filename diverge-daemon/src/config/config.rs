//! The document itself.

use diverge_sdk::daemon::DEFAULT_PORT;
use diverge_sdk::shared::Identity;
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
}

/// What a daemon runs on before it has written a line of
/// configuration: the port one above the provider's, no identity, no
/// administrator, a Postgres of its own, and ten seconds of idleness.
/// What it announces itself with, whether it accepts other daemons and
/// whether it hears messages are each provider's record's to say, and
/// no document's.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: DEFAULT_PORT,
            identity: None,
            admins: Vec::new(),
            postgres: Postgres::default(),
            idle_seconds: 10,
        }
    }
}
