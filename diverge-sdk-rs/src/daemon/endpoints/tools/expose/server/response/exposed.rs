//! What an exposure hands a connecting daemon.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// Everything a connecting daemon needs to join the tool's container:
/// the provider it runs on, as this daemon names it, and the identity
/// this daemon is known by there — the connector matches it against
/// the links of its record of this daemon, and joins through the link
/// that names it — the container's id, and the authorization the
/// provider protocol's `containers::tools::connect` presents, which
/// this daemon judges by the exposure and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Exposed {
    /// The provider the container runs on: see [`Identity`].
    pub provider: Identity,
    /// The identity this daemon is known by at that provider when it
    /// accepts daemon connections through it, as the provider answered
    /// its accept; absent when it accepts none there, and then no
    /// daemon reaches the tool through that provider by a link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    /// The container's id, as the provider's run answered it: what the
    /// provider protocol's connect names a container by.
    pub id: String,
    /// The authorization: minted for this exposure alone, answered
    /// here and never again, held in memory and on no record, and
    /// spent by the one connection that presents it. A second
    /// presentation is declined, and so is any after the scope ends.
    pub authorization: String,
}
