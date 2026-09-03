//! What a turn accumulates beyond the conversation.

use serde::{Deserialize, Serialize};

/// The memory-shaping switches, every one stated.
///
/// Each names a runtime construction fact — a constructor option or
/// a character flag — that adds model calls to a turn and rows to
/// the continuation; together they are what makes an Eliza agent an
/// Eliza agent rather than a chat loop, which is why they are the
/// caller's and not the harness's. All plain booleans, none
/// optional: a request says what its turns cost.
///
/// Not a switch, because nothing switches it: the per-turn FACTS
/// stage. The message handler's first pass extracts facts and
/// relationships as part of its one structured envelope, and they
/// are persisted whenever it found any — a cost already inside the
/// turn the caller asked for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
pub struct Memory {
    /// The advanced-capabilities bundle: the post-turn reflection
    /// evaluators (fact memory, relationships, identities, success),
    /// experience, personality and form providers and actions — one
    /// merged small-model call after each reply. Constructor option
    /// `advancedCapabilities`.
    pub advanced_capabilities: bool,
    /// Long-term memory: distilled episodic, semantic and procedural
    /// memories in their own table, written by the advanced-memory
    /// evaluator. Character flag `advancedMemory`.
    pub advanced_memory: bool,
    /// The relationships feature: the native entity graph, its
    /// service and provider. Constructor option `enableRelationships`.
    pub relationships: bool,
    /// Advanced planning: the multi-step planner over the simple
    /// reply path. Character flag `advancedPlanning`.
    pub advanced_planning: bool,
}
