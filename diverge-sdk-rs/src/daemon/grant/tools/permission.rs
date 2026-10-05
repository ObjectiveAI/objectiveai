//! One grant's worth over tools.

use serde::{Deserialize, Serialize};

use crate::daemon::grant::{Tagging, Within};
use crate::daemon::endpoints::tools::list::client::request::Filter;
use super::{Make, Over};

/// What one grant over tools allows, in one of three shapes told apart
/// on the wire by their very form, as [`grant`](crate::daemon::grant)
/// states them: a bare array of [`Make`] actions; an object of [`Over`]
/// actions and how far they reach; or an object of tagging actions, how
/// far they reach, and which tags. An object that is none of these — an
/// action of one shape among another's, a member missing — does not
/// decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over tools that exist: the [`Over`] actions held, reaching every
    /// one of them, or only those the [`Filter`] lets through, read as
    /// a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
    },
    /// Tagging tools: putting tags on, taking them off, or both,
    /// reaching every one or only those the [`Filter`] lets through,
    /// and touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
        /// Which tags: `"any"`, or only these.
        tags: Within<Vec<String>>,
    },
}
