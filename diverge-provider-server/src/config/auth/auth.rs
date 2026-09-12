//! The section itself.

use serde::{Deserialize, Serialize};

use super::Unbrokered;

/// The `auth` section: how a peer that dials the provider is judged.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Auth {
    /// The ways an unbrokered credential is judged, tried in order:
    /// the first that accepts it decides. Empty means no dialling
    /// peer is accepted.
    pub unbrokered: Vec<Unbrokered>,
}
