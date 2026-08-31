//! Searching past conversations.

use serde::{Deserialize, Serialize};

/// The `session_search` switch.
///
/// Untagged: a bool is the bare switch, an object is the switch
/// thrown on with arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Toolset {
    /// `false` = explicitly off; `true` = on with nothing supplied.
    Switch(bool),
    /// On, with arguments. See [`Config`].
    Config(Config),
}

/// Searching past conversations.
///
/// Nothing to supply: the search runs over the local session
/// database, credential-free. In a one-run container the
/// searchable past is this run's own. The empty object and
/// `true` say the same thing.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {}
