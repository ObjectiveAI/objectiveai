//! Stop sequences.

use serde::{Deserialize, Serialize};

/// Stop sequences that terminate model generation.
///
/// When the model generates any of these sequences, it immediately
/// stops producing further tokens.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[serde(untagged)]
pub enum Stop {
    /// A single stop sequence.
    String(String),
    /// Multiple stop sequences (up to 4 typically supported).
    Strings(Vec<String>),
}
