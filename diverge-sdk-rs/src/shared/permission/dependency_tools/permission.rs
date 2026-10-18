//! One grant's worth over dependency templates, naming none of them.

use serde::{Deserialize, Serialize};

use crate::shared::permission::{Tagging, Tags, Within};
use super::Over;

/// What one grant over dependency templates allows, in one of two
/// shapes told apart on the wire by their very form, as
/// [`permission`](crate::shared::permission) states them: an object
/// of [`Over`] actions and how far they reach, by tags; or an object
/// of tagging actions, how far they reach, and which tags. There is
/// no making shape: nothing on the wire makes a dependency template.
/// An object that is neither — an action of one shape among the
/// other's, a member missing, a `within` that is not `"any"` or a
/// [`Tags`] — does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// Over dependency templates that exist: the [`Over`] actions
    /// held, reaching every one of them, or only those the [`Tags`]
    /// let through, read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the tags as a test.
        within: Within<Tags>,
    },
    /// Tagging dependency templates: putting tags on, taking them off,
    /// or both, reaching every one or only those the [`Tags`] let
    /// through, and touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the tags as a test.
        within: Within<Tags>,
        /// Which tags: `"any"`, or only these.
        tags: Within<Vec<String>>,
    },
}
