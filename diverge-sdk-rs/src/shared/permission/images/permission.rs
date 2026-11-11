//! One grant's worth over images.

use serde::{Deserialize, Serialize};

use super::Action;

/// The actions held over images, a bare array on the wire:
/// `["info"]`. An image is named by its digest and is no record the
/// daemon keeps, so there is nothing to narrow and no `within`; an
/// action is held or it is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(transparent)]
pub struct Permission(pub Vec<Action>);
