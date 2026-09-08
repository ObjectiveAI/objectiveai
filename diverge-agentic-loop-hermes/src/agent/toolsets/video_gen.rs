//! Video generation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Video generation over FAL, the built-in path, with its
/// arguments — the same deliberate one-provider scope as
/// [`image_gen`](super::image_gen). The field being absent from
/// [`Toolsets`](super::Toolsets) is Hermes's own default for this
/// toolset; present is the switch thrown on.
///
/// The key is REQUIRED: generation has no keyless path, so a
/// switch thrown on without one could only mean a dead toolset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Toolset {
    /// The FAL key, applied as `FAL_KEY` in the gateway's process
    /// environment. [`image_gen`](super::image_gen) names the same
    /// variable; a request supplying both MUST agree with itself.
    pub fal_key: String,
}
