//! One grant's worth over dependency templates.

use serde::{Deserialize, Serialize};

use crate::daemon::grant::{Tagging, Within};
use crate::daemon::endpoints::dependencies::list::client::request::Filter;
use super::Over;

/// What one grant over dependency templates allows, in one of two
/// shapes told apart on the wire by their very form, as
/// [`grant`](crate::daemon::grant) states them: an object of [`Over`]
/// actions and how far they reach; or an object of tagging actions,
/// how far they reach, and which tags. An object that is neither — an
/// action of one shape among the other's, a member missing — does not
/// decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// Over dependency templates that exist: the [`Over`] actions
    /// held, reaching every one of them, or only those the [`Filter`]
    /// lets through, read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
    },
    /// Tagging dependency templates: putting tags on, taking them off,
    /// or both, reaching every one or only those the [`Filter`] lets
    /// through, and touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the filter as a test.
        within: Within<Filter>,
        /// Which tags: `"any"`, or only these.
        tags: Within<Vec<String>>,
    },
}
