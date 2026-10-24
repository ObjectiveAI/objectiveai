//! One grant's worth over the events.

use serde::{Deserialize, Serialize};

use super::Action;

/// The actions held over the events, a bare array on the wire:
/// `["list"]`. There is one graph of events, so there is no `within`;
/// an action is held or it is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
