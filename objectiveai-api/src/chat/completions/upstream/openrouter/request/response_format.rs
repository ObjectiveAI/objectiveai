//! Response format construction for vector completions.

use crate::vector;

/// Creates a response format for vector completion voting.
///
/// When the output mode is `JsonSchema`, returns a JSON schema that constrains
/// the LLM's output to select one of the available response keys.
/// Returns None for other output modes.
pub fn new_for_vector(
    vector_pfx_indices: &[(String, usize)],
    ensemble_llm_output_mode: objectiveai::ensemble_llm::OutputMode,
    ensemble_llm_synthetic_reasoning: Option<bool>,
) -> Option<objectiveai::chat::completions::request::ResponseFormat> {
    if let objectiveai::ensemble_llm::OutputMode::JsonSchema = ensemble_llm_output_mode {
        Some(vector::completions::ResponseKey::response_format(
            vector_pfx_indices
                .iter()
                .map(|(key, _)| key.clone())
                .collect(),
            ensemble_llm_synthetic_reasoning.unwrap_or(false),
        ))
    } else {
        None
    }
}
