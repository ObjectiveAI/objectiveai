//! One grant's worth over providers.

use serde::{Deserialize, Serialize};

use super::{Make, Over, Reach};
use crate::daemon::grant::{Tagging, Within};

/// What one grant over providers allows, in one of three shapes told
/// apart on the wire by their very form, as
/// [`grant`](crate::daemon::grant) states them: a bare array of
/// [`Make`] actions; an object of [`Over`] actions and how far they
/// reach; or an object of tagging actions, how far they reach, and
/// which tags. An object that is none of these does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over providers' records: the [`Over`] actions held, reaching
    /// every one of them, or only those the [`Reach`] lets through —
    /// by identity, by tags, or both.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the reach as a test.
        within: Within<Reach>,
    },
    /// Tagging providers' records: putting tags on, taking them off, or
    /// both, reaching every one or only those the [`Reach`] lets
    /// through, and touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the reach as a test.
        within: Within<Reach>,
        /// Which tags: `"any"`, or only these.
        tags: Within<Vec<String>>,
    },
}
