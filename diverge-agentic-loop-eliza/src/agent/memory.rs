//! What a turn accumulates beyond the conversation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The memory-shaping switches, every one stated.
///
/// Each names a runtime construction fact — a constructor option or
/// a character flag — that adds model calls to a turn and rows to
/// the continuation; together they are what makes an Eliza agent an
/// Eliza agent rather than a chat loop, which is why they are the
/// caller's and not the harness's. Each an `Option<bool>` where
/// absent is off — the same as `false` — so a request names only
/// what it pays for.
///
/// Not a switch, because nothing switches it: the per-turn FACTS
/// stage. The message handler's first pass extracts facts and
/// relationships as part of its one structured envelope, and they
/// are persisted whenever it found any — a cost already inside the
/// turn the caller asked for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default,
)]
pub struct Memory {
    /// The advanced-capabilities bundle: the post-turn reflection
    /// evaluators (fact memory, relationships, identities, success),
    /// experience, personality and form providers and actions — one
    /// merged small-model call after each reply. Constructor option
    /// `advancedCapabilities`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_capabilities: Option<bool>,
    /// Long-term memory: distilled episodic, semantic and procedural
    /// memories in their own table, written by the advanced-memory
    /// evaluator. Character flag `advancedMemory`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_memory: Option<bool>,
    /// The relationships feature: the native entity graph, its
    /// service and provider. Constructor option `enableRelationships`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationships: Option<bool>,
    /// Advanced planning: the multi-step planner over the simple
    /// reply path. Character flag `advancedPlanning`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_planning: Option<bool>,
}
