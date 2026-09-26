//! The one provider an agent runs on, and the volumes it mounts
//! there.

use serde::{Deserialize, Serialize};

use super::VolumeMount;
use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// Pin the agent to one provider, and mount that provider's volumes
/// in it.
///
/// A volume is a provider's own: named in that provider's listing,
/// kept on that provider's disk, and meaning nothing to any other.
/// So an agent that mounts volumes can run on one provider only, the
/// one the volumes belong to, and this is where a create says so —
/// the provider by its [`Identity`], as the daemon knows it, and the
/// mounts by that provider's names. An agent created without one
/// runs on whichever provider the daemon chooses, and mounts nothing:
/// its state is its conversation, which the daemon keeps, and
/// nothing on any provider's disk.
///
/// # The identity is the daemon's
///
/// The same object the agent's log names a provider by, in an
/// [`active`](crate::daemon::endpoints::agents::logs::server::response::Active)
/// item: `kind: "outgoing"` and the address the daemon dials, or
/// `kind: "incoming_unbrokered"` and the identity the daemon's
/// judging of the provider's credential answered. A provider the
/// daemon does not know by that identity is the create's error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provider {
    /// Which provider. See [`Identity`].
    pub identity: Identity,
    /// Volumes of that provider, made visible inside the container.
    ///
    /// Ordered, and applied in order. See [`VolumeMount`] for how one
    /// is named without a host path; the `volume_name` is a name in the
    /// daemon's listing from THAT provider. No mount's path, in any
    /// list of the create, is a prefix of another's: mounting INTO a
    /// directory the image owns is the point, and mounts stacking on
    /// each other is not. Empty pins the agent to the provider and
    /// mounts nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_mounts: Vec<VolumeMount>,
}
