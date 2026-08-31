//! Home Assistant control.

use serde::{Deserialize, Serialize};

/// Home Assistant control, with its arguments. The field being
/// absent from [`Toolsets`](super::Toolsets) is Hermes's own
/// default for this toolset; present is the switch thrown on.
///
/// Both arguments are REQUIRED here even though Hermes treats the
/// url as optional: its fallback is `homeassistant.local`, an mDNS
/// name that is dead inside a container — an instance the caller
/// does not name is an instance this run cannot reach.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Toolset {
    /// The instance, applied as `HASS_URL` in the gateway's
    /// process environment.
    pub url: String,
    /// The long-lived access token, applied as `HASS_TOKEN` —
    /// static by Home Assistant's own design, never rotated.
    pub token: String,
}
