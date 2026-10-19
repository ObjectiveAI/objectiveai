//! One grant's worth over agent templates, naming none of them.

use serde::{Deserialize, Serialize};

use crate::shared::permission::{Tagging, Tags, Within};
use super::{Make, Over};

/// What one grant over agent templates allows, in one of three shapes told
/// apart on the wire by their very form, as
/// [`permission`](crate::shared::permission) states them: a bare array
/// of [`Make`] actions; an object of [`Over`] actions and how far they
/// reach, by tags; or an object of tagging actions, how far they
/// reach, and which tags. An object that is none of these — an action
/// of one shape among another's, a member missing, a `within` that is
/// not `"any"` or a [`Tags`] — does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over agent templates that exist: the [`Over`] actions held, reaching
    /// every one of them, or only those the [`Tags`] let through,
    /// read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the tags as a test.
        within: Within<Tags>,
    },
    /// Tagging agent templates: putting tags on, taking them off, or both,
    /// reaching every one or only those the [`Tags`] let through, and
    /// touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the tags as a test.
        within: Within<Tags>,
        /// Which tags: `"any"`, or only these.
        tags: Within<Vec<String>>,
    },
}
