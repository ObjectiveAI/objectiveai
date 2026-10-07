//! The document that is `config.yaml`.

use diverge_sdk::container_proxy::outside::OUTSIDE_PORT;
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
}

/// What a daemon runs on before it has written a line of
/// configuration: the port one above the provider's.
impl Default for Config {
    fn default() -> Self {
        Config { port: OUTSIDE_PORT + 1 }
    }
}
