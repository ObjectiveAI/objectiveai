//! The `prompt_suggestion` records: what to ask next.

use serde::{Deserialize, Serialize};

/// A `type: "prompt_suggestion"` record: a suggested follow-up,
/// emitted after the result when the run opted into suggestions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptSuggestion {
    /// The suggested prompt.
    pub suggestion: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}
