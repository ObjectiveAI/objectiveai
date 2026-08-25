//! Stop sequences.

use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::openrouter;
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

/// The provider request's stop sequences, field for field.
impl From<openrouter::Stop> for Stop {
    fn from(stop: openrouter::Stop) -> Self {
        match stop {
            openrouter::Stop::String(stop) => Stop::String(stop),
            openrouter::Stop::Strings(stops) => Stop::Strings(stops),
        }
    }
}
