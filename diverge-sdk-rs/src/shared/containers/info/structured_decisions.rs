//! What an agent that makes structured decisions says of itself.

use serde::{Deserialize, Serialize};

use super::Modalities;

/// How an agent's program makes structured decisions: two members,
/// both required, and no other.
///
/// Stated by an agent's image, in the frontmatter's
/// `structured_decisions`. Nothing reads it yet: there is no request
/// that asks an agent for a structured decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StructuredDecisions {
    /// What the prompt of a decision may carry beside text.
    pub prompt_modalities: Modalities,
    /// What a choice of a decision may carry beside text.
    pub choice_modalities: Modalities,
}
