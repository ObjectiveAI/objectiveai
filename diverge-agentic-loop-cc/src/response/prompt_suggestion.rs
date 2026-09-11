//! The `prompt_suggestion` records: what to ask next.

use serde::Deserialize;

/// A `type: "prompt_suggestion"` record: a suggested follow-up,
/// emitted after the result when the run opted into suggestions.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PromptSuggestion {
    /// Always `prompt_suggestion`.
    pub r#type: PromptSuggestionType,
    /// The suggested prompt.
    pub suggestion: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `prompt_suggestion` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptSuggestionType {
    /// The only value.
    #[default]
    PromptSuggestion,
}
