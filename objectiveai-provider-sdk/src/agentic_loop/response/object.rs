//! Object discriminator for agentic loop chunks.

use serde::{Deserialize, Serialize};

/// The object type carried by every [`AgenticLoopChunk`](super::AgenticLoopChunk).
///
/// A single-variant enum rather than a bare string: the value is fixed,
/// and making it a type means a wrong one fails to deserialize instead
/// of being carried around as data nobody checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Object {
    /// An agentic loop chunk.
    #[serde(rename = "agentic_loop.chunk")]
    #[default]
    AgenticLoopChunk,
}
