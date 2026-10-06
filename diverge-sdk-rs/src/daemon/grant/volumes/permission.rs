//! One grant's worth over volumes.

use serde::{Deserialize, Serialize};

use crate::daemon::grant::Within;
use crate::daemon::endpoints::volumes::list::client::request::Filter;
use super::{Make, Over};

/// What one grant over volumes allows, in one of two shapes told apart
/// on the wire by their very form, as [`grant`](crate::daemon::grant)
/// states them: a bare array of [`Make`] actions; or an object of
/// [`Over`] actions and how far they reach. An object that is none of
/// these — an action of one shape among another's, a member missing —
/// does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over volumes that exist: the [`Over`] actions held, reaching
    /// every one of them, or only those the [`Filter`] lets through,
    /// read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
    },
}
