//! OpenRouter upstream marker.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// [`Agent`](super::Agent)'s discriminator.
///
/// One variant: an agent value whose `upstream` is anything else is
/// not this image's, and fails to parse as one.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Upstream {
    #[default]
    Openrouter,
}
