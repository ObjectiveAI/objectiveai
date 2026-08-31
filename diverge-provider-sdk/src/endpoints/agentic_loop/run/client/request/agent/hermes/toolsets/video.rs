//! Video analysis.

use serde::{Deserialize, Serialize};

/// The `video` switch.
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

/// Video analysis.
///
/// Nothing to supply: analysis rides the run's own inference
/// provider, exactly as [`vision`](super::vision) does. The
/// empty object and `true` say the same thing.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {}
