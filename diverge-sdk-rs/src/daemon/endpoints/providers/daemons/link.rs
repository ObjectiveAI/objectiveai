//! One way a daemon is reached: a provider, and its name for the
//! daemon.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// One link of a daemon record: a provider of the caller's, by the
/// identity this daemon knows it by, and the identity the remote daemon
/// is known by at that provider — what the provider protocol's
/// `daemons::connect` names an acceptor by there. A daemon's links are
/// every provider it is reachable through; a connection to it is opened
/// through any one whose provider this daemon is connected to, and a
/// tool it exposes is joined through the one whose `identity` the
/// expose answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Link {
    /// The provider, as this daemon names one: see [`Identity`]. One of
    /// the caller's, outgoing or incoming, on record.
    pub provider: Identity,
    /// The remote daemon's identity at that provider: what its
    /// connection there is authorized under, which the provider answers
    /// it when it accepts daemon connections and it reports in an
    /// expose.
    pub identity: String,
}
