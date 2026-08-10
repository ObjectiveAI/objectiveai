//! The script agent.

use serde::{Deserialize, Serialize};

use super::{Script, Upstream};

/// An agent that runs code instead of calling a model.
///
/// No `model` field, which is the point: nothing is sampled, so there
/// is nothing to name. A script agent occupies the same slot as a
/// model-backed one and answers deterministically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// The discriminator. Always `script`.
    pub upstream: Upstream,
    /// The code to run.
    pub script: Script,
}
