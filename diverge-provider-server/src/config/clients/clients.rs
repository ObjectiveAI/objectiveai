//! The section itself.

use serde::{Deserialize, Serialize};

use super::Unbrokered;

/// The `clients` section: the peers the provider dials.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Clients {
    /// The peers dialled in the unbrokered mode. Empty means the
    /// provider dials no one.
    pub unbrokered: Vec<Unbrokered>,
}
