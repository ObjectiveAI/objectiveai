//! The document itself.

use serde::{Deserialize, Serialize};

use super::Daemon;

/// The whole of `viewer.yaml`: what the viewer is told.
///
/// An absent file is this type's [`Default`]. An unknown key is an
/// error, so a misspelled setting is refused rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The daemon the viewer dials, and as whom: see [`Daemon`]. Read
    /// at start; another daemon is another start. Absent means the
    /// daemon on this host, as its root account.
    pub daemon: Daemon,
}

/// What a viewer runs against before it has written a line of
/// configuration: the daemon on this host, as its root account.
impl Default for Config {
    fn default() -> Self {
        Config { daemon: Daemon::default() }
    }
}
