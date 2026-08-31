//! Video generation.

use serde::{Deserialize, Serialize};

/// Video generation over FAL, the built-in path, with its
/// arguments — the same deliberate one-provider scope as
/// [`image_gen`](super::image_gen). The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
    /// The FAL key, applied as `FAL_KEY` in the gateway's process
    /// environment. [`image_gen`](super::image_gen) names the same
    /// variable; a request supplying both MUST agree with itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fal_key: Option<String>,
}
