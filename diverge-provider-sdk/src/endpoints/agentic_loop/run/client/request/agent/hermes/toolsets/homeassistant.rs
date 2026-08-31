//! Home Assistant control.

use serde::{Deserialize, Serialize};

/// The `homeassistant` switch.
///
/// Untagged: a bool is the bare switch, an object is the switch
/// thrown on with arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Toolset {
    /// `false` = explicitly off; `true` = on with nothing supplied.
    Switch(bool),
    /// On, with arguments. See [`Config`].
    Config(Config),
}

/// Home Assistant control. Both arguments are REQUIRED here even
/// though Hermes treats the url as optional: its fallback is
/// `homeassistant.local`, an mDNS name that is dead inside a
/// container — an instance the caller does not name is an instance
/// this run cannot reach.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    /// The instance, applied as `HASS_URL` in the gateway's
    /// process environment.
    pub url: String,
    /// The long-lived access token, applied as `HASS_TOKEN` —
    /// static by Home Assistant's own design, never rotated.
    pub token: String,
}
