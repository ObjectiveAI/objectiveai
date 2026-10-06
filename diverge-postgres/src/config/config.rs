//! The document that is `config.yaml`.

use serde::{Deserialize, Serialize};

/// The whole of `config.yaml`.
///
/// An absent file is this type's [`Default`]. An unknown key is an
/// error, so a misspelled setting is refused rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The most connections the postmaster accepts at once, passed to
    /// it as `max_connections`. Every container connection the daemon
    /// splices through is one of them, so this is sized for a daemon
    /// with many containers rather than for a desktop.
    pub max_connections: u32,
}

/// What the supervisor runs on before it has written a line of
/// configuration: a thousand and twenty-four connections.
impl Default for Config {
    fn default() -> Self {
        Config { max_connections: 1024 }
    }
}
