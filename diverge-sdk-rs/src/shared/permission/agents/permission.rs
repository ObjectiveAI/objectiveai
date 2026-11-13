//! One grant's worth over agents.

use serde::{Deserialize, Serialize};

use crate::shared::permission::{self, Tagging, Tags};
use super::{Make, Over, Within};

/// What one grant over agents allows, in one of three shapes told
/// apart on the wire by their very form, as
/// [`permission`](crate::shared::permission) states them: a bare array
/// of [`Make`] actions; an object of [`Over`] actions and how far they
/// reach — every agent, the tool's attached agents, those never active,
/// the agents listed, or by tags; or an object
/// of tagging actions, how far they reach, and which tags. An object
/// that is none of these — an action of one shape among another's, a
/// member missing, a `within` that is not `"any"`, `"attached"`,
/// `"never_active"`, `{"agents":[…]}` or a [`Tags`] — does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum Permission {
    /// To make: the [`Make`] actions held, as a bare array. Nothing is
    /// judged but that the action is held.
    Make(Vec<Make>),
    /// Over agents that exist: the [`Over`] actions held, reaching
    /// every one of them, the tool's attached agents, those never active,
    /// the agents listed, or only those
    /// the [`Tags`] let through, read as a test.
    Over {
        /// The actions.
        actions: Vec<Over>,
        /// How far: `"any"`, `"attached"`, `"never_active"`, the agents
        /// listed, or the tags as a test. See
        /// [`Within`].
        within: Within,
    },
    /// Tagging agents: putting tags on, taking them off, or both,
    /// reaching every one or only those the [`Tags`] let through, and
    /// touching any tag or only the ones listed.
    Tags {
        /// `tag`, `untag`, or both.
        actions: Vec<Tagging>,
        /// How far: `"any"`, or the tags as a test.
        within: permission::Within<Tags>,
        /// Which tags: `"any"`, or only these.
        tags: permission::Within<Vec<String>>,
    },
}
