//! The block itself.

use serde::{Deserialize, Serialize};

use super::Postgres;
use crate::container_proxy::outside::OUTSIDE_PORT;

/// The `daemon` block of `config.yaml`: the whole of what the daemon
/// is told.
///
/// An absent block is this type's [`Default`]. An unknown key is an
/// error, so a misspelled setting is refused rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The TCP port the daemon accepts the protocol on: a WebSocket
    /// upgrade at any path, on every interface. Absent means one above
    /// the port a provider listens on, so that a daemon and a provider
    /// on one host do not contend for it.
    pub port: u16,
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
    /// connection it holds, and judges each daemon that connects as it
    /// judges any client, by the credential presented. When `false`,
    /// it opens none, and no daemon reaches it or the tools it exposes.
    /// Absent means `true`.
    pub accept_daemons: bool,
}

/// What a daemon runs on before it has written a line of
/// configuration: the port one above the provider's, a Postgres of
/// its own, ten seconds of idleness, and other daemons accepted.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: OUTSIDE_PORT + 1,
            postgres: Postgres::default(),
            idle_seconds: 10,
            accept_daemons: true,
        }
    }
}
