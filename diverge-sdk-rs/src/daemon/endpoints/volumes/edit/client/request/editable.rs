//! The volume as a patch sees it.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::volumes::Mode;

/// The members of a volume an edit may change, as one JSON document:
/// its size and its mode, as the provider lists them, and as the
/// result must be again. The name does not change: it is the handle,
/// and a mount naming it would name nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// How many bytes it reserves: larger reserves more, smaller gives
    /// room back and is refused when the volume holds more than that.
    pub bytes: u64,
    /// The mode it is in: see [`Mode`]. The content is untouched
    /// either way.
    pub mode: Mode,
}
