//! Image generation.

use serde::{Deserialize, Serialize};

/// Image generation over FAL, the built-in path, with its
/// arguments. The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on. Hermes's other image
/// providers (openai, krea, deepinfra, xai) are deliberately
/// absent at this pin — one keyed path is enough until asked for
/// more.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Toolset {
    /// The FAL key, applied as `FAL_KEY` in the gateway's process
    /// environment. [`video_gen`](super::video_gen) names the same
    /// variable; a request supplying both MUST agree with itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fal_key: Option<String>,
}
