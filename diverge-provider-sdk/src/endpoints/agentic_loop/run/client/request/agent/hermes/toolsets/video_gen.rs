//! Video generation.

use serde::{Deserialize, Serialize};

/// The `video_gen` switch.
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

/// Video generation over FAL, the built-in path — the same
/// deliberate one-provider scope as
/// [`image_gen`](super::image_gen).
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Config {
    /// The FAL key, applied as `FAL_KEY` in the gateway's process
    /// environment. [`image_gen`](super::image_gen) names the same
    /// variable; a request supplying both MUST agree with itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fal_key: Option<String>,
}
