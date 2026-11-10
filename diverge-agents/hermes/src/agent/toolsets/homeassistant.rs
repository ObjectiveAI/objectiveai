//! Home Assistant control.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Home Assistant control, with its arguments. The field being
/// absent from [`Toolsets`](super::Toolsets) is Hermes's own
/// default for this toolset; present is the switch thrown on.
///
/// The url is REQUIRED here even though Hermes treats it as
/// optional: its fallback is `homeassistant.local`, an mDNS name
/// that is dead inside a container — an instance the caller does
/// not name is an instance this run cannot reach.
///
/// SECRET: `HASS_TOKEN`, fixed — the long-lived access token, static
/// by Home Assistant's own design, never rotated; applied in the
/// gateway's process environment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Toolset {
    /// The instance, applied as `HASS_URL` in the gateway's
    /// process environment.
    pub url: String,
}
