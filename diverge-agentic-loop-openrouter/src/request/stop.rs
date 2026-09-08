//! Stop sequences.

use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
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
impl From<openrouter::Stop> for Stop {
    fn from(stop: openrouter::Stop) -> Self {
        match stop {
            openrouter::Stop::String(stop) => Stop::String(stop),
            openrouter::Stop::Strings(stops) => Stop::Strings(stops),
        }
    }
}
