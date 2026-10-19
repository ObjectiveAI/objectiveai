//! One grant's worth over the database.

use serde::{Deserialize, Serialize};

use super::Action;

/// The actions held over the database, a bare array on the wire:
/// `["get","connections"]`. There is one database, so there is no
/// `within`; an action is held or it is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
