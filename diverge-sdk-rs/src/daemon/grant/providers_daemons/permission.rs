//! One grant's worth through providers.

use serde::{Deserialize, Serialize};

use super::Over;
use crate::daemon::grant::Within;
use crate::daemon::grant::providers::Reach;

/// What one grant through providers allows, in the one shape
/// [`grant`](crate::daemon::grant) states for actions over what
/// exists: an object of [`Over`] actions and how far they reach. An
/// object that is not this does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// Through providers: the [`Over`] actions held, reaching every
    /// provider, or only those the [`Reach`] lets through — by
    /// identity, by their records' tags, or both.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the reach as a test.
        within: Within<Reach>,
    },
}
