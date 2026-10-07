//! The document that is `config.yaml`.

use diverge_sdk::container_proxy::outside::OUTSIDE_PORT;
use diverge_sdk::daemon::endpoints::postgres::Mode;
use serde::{Deserialize, Serialize};

/// The whole of `config.yaml`.
///
/// An absent file is this type's [`Default`]. An unknown key is an
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
    /// containers, as the wire's [`Mode`] states it: `{kind: local}`,
    /// a Postgres the daemon runs beside itself, or `{kind: remote,
    /// url: …}`, one it dials. Read at start and never changed while
    /// the daemon runs: another database is another start. Absent
    /// means local.
    pub postgres: Mode,
}

/// What a daemon runs on before it has written a line of
/// configuration: the port one above the provider's, and a Postgres
/// of its own.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: OUTSIDE_PORT + 1,
            postgres: Mode::Local,
        }
    }
}
