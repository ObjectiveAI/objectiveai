//! A grant over providers.

use serde::{Deserialize, Serialize};

use super::Action;

/// The acts a grant allows over every provider of the daemon's: as
/// JSON, the list of them, `["get", "list"]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
