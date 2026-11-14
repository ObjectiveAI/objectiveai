//! One grant's worth over images.

use serde::{Deserialize, Serialize};

use super::Action;
use crate::daemon::grant::Within;
use crate::daemon::grant::providers::Reach;

/// What one grant over images allows, in the one shape
/// [`grant`](crate::daemon::grant) states for actions over what
/// exists: an object of [`Action`]s and how far they reach. An object
/// that is not this does not decode, and neither does a bare array of
/// actions, which reached every provider and is no shape of this
/// grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// Over the daemon's Stores: the [`Action`]s held, reaching every
    /// provider's, or only those of the providers the [`Reach`] lets
    /// through — by identity, by their records' tags, or both.
    Over {
        /// The actions.
        actions: Vec<Action>,
        /// How far: `"any"`, or the reach as a test.
        within: Within<Reach>,
    },
}
