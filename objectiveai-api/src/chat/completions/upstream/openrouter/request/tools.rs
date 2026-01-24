//! Tools construction for vector completions.

use crate::vector;

/// Creates tools for vector completion voting.
///
/// When the output mode is `ToolCall`, creates a tool that the LLM must call
/// to select one of the available response keys. The tool is appended to any
/// existing tools from the request.
/// Returns None for other output modes.
pub fn new_for_vector(
    vector_pfx_indices: &[(String, usize)],
    ensemble_llm_output_mode: objectiveai::ensemble_llm::OutputMode,
    ensemble_llm_synthetic_reasoning: Option<bool>,
    request: Option<&[objectiveai::chat::completions::request::Tool]>,
) -> Option<Vec<objectiveai::chat::completions::request::Tool>> {
    if let objectiveai::ensemble_llm::OutputMode::ToolCall = ensemble_llm_output_mode {
        let tool = vector::completions::ResponseKey::tool(
            vector_pfx_indices
                .iter()
                .map(|(key, _)| key.clone())
                .collect(),
            ensemble_llm_synthetic_reasoning.unwrap_or(false),
        );
        Some(match request {
            Some(request) => {
                let mut tools = Vec::with_capacity(request.len() + 1);
                tools.extend_from_slice(request);
                tools.push(tool);
                tools
            }
            None => vec![tool],
        })
    } else {
        None
    }
}
