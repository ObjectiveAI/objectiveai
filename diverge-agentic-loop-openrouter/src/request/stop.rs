//! Stop sequences.

use crate::agent;
use serde::Serialize;

/// Stop sequences that terminate model generation.
///
/// When the model generates any of these sequences, it immediately
/// stops producing further tokens.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
)]
#[serde(untagged)]
pub enum Stop {
    /// A single stop sequence.
    String(String),
    /// Multiple stop sequences (up to 4 typically supported).
    Strings(Vec<String>),
}

/// The provider request's stop sequences, field for field.
impl From<agent::Stop> for Stop {
    fn from(stop: agent::Stop) -> Self {
        match stop {
            agent::Stop::String(stop) => Stop::String(stop),
            agent::Stop::Strings(stops) => Stop::Strings(stops),
        }
    }
}
