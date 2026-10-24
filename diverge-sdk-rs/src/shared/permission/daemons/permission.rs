//! A grant over remote daemons.

use serde::{Deserialize, Serialize};

use super::Action;

/// The acts a grant allows over every remote daemon this daemon
/// reaches: as JSON, the list of them, `["get", "relay"]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
