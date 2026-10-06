//! Naming a volume: its provider, and its name there.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// One volume: the provider that holds it, by the daemon's [`Identity`]
/// for it, and the name that provider lists it under. A volume is a
/// provider's own and has no name anywhere else, so the pair is the
/// whole of how one is named, on every request that acts on one and in
/// every place one is mounted. One JSON object,
/// `{"provider":…,"name":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Volume {
    /// The provider: `kind: "outgoing"` and the address the daemon
    /// dials, or `kind: "incoming_unbrokered"` and the identity its
    /// judging answered. See [`Identity`].
    pub provider: Identity,
    /// The name the provider lists the volume under, as a
    /// [`list`](crate::daemon::endpoints::volumes::list) reports it;
    /// compared and not read.
    pub name: String,
}
