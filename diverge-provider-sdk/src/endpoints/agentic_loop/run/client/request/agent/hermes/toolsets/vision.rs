//! Image analysis.

use serde::{Deserialize, Serialize};

/// The `vision` switch.
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

/// Image analysis.
///
/// Nothing to supply: analysis rides the run's own inference
/// provider — Hermes's auxiliary chain starts at the agent's
/// provider, so the provider's auth is the whole of it. The
/// empty object and `true` say the same thing.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {}
