//! One daemon, as this daemon holds it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::daemons::Link;
use crate::shared::listing::Keyed;

/// One remote daemon of the configuration's: its name, the issuer this
/// daemon presents as to it, its links, and whether this daemon holds a
/// connection to it now.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Daemon {
    /// The name, as the configuration gives it.
    pub name: String,
    /// The issuer this daemon presents as to it: the remote holds this
    /// daemon's identity there as one of its accounts.
    pub issuer: String,
    /// The providers it is reached through, each with the identity it
    /// is known by there: see [`Link`]. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Whether this daemon holds a connection to it now.
    pub connected: bool,
}

/// Keyed by the name.
impl Keyed for Daemon {
    type Key = String;

    fn key(&self) -> String {
        self.name.clone()
    }
}
