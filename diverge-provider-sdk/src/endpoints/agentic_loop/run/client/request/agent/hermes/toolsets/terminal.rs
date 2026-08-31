//! Terminal and process control.

use serde::{Deserialize, Serialize};

/// The `terminal` switch.
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

/// Terminal and process control.
///
/// Nothing to supply: the container is the sandbox, and the
/// local backend reads no credentials. (Hermes's remote
/// backends — ssh, daytona, vercel, modal — are deliberately
/// absent: a container does not outsource its own shell.)
/// The empty object and `true` say the same thing.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {}
