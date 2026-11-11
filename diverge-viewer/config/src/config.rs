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
    /// at start; another daemon is another start. Absent, the viewer
    /// does not start, since it has no identity to present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon: Option<Daemon>,
}

/// A document with no line in it: no daemon, so a viewer refuses to
/// start until one is named.
impl Default for Config {
    fn default() -> Self {
        Config { daemon: None }
    }
}
