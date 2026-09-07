//! Codex upstream marker.

use serde::{Deserialize, Serialize};

/// [`Agent`](super::Agent)'s discriminator.
///
/// One variant, and the reason [`Agent`](super::super::Agent) can be
/// untagged: no other upstream's agent can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Upstream {
    #[default]
    Codex,
}
