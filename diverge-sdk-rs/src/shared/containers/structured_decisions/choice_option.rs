//! One option of a choice.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

/// One option of a [`Question::Choice`](super::Question::Choice): its
/// value, and perhaps a description. No other member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChoiceOption {
    /// The option's name, text alone: what its probability is keyed by
    /// in the answer.
    pub value: String,
    /// What the option means, as content parts, in order. Absent, the
    /// value says it all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<ContentBlock>>,
}
