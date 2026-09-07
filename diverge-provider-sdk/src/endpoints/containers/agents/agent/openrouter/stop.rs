//! Stop sequences.

use serde::{Deserialize, Serialize};

/// Sequences that end generation when produced.
///
/// Untagged: one sequence goes on the wire as a bare string and
/// several as an array, matching what providers accept without a
/// caller having to wrap a single sequence in a list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    /// One sequence.
    String(String),
    /// Several. Providers typically cap this around four.
    Strings(Vec<String>),
}
