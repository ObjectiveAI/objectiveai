//! One grant's worth over remote daemons.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::daemons::list::client::request::Filter;
use crate::daemon::grant::{Tagging, Within};
use super::{Make, Over};

/// What one grant over remote daemons allows, in one of three
/// shapes told apart on the wire by their very form, as
/// [`grant`](crate::daemon::grant) states them: a bare array of
/// [`Make`] actions; or an object of [`Over`] actions and how far they
/// reach; or an object of tagging actions, how far they reach, and
/// which tags. An object that is none of these does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over remote daemons' records: the [`Over`] actions held, reaching
    /// every one of them, or only those the [`Filter`] lets through,
    /// read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
    },
    /// Tagging remote daemons' records: putting tags on, taking them off, or both,
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
