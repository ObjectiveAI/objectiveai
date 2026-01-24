//! Tool choice construction for vector completions.

use crate::vector;

/// Creates tool choice configuration for vector completion voting.
///
/// When the output mode is `ToolCall`, returns the tool choice that forces
/// the LLM to call the response selection tool.
/// When there are request tools but output mode is not `ToolCall`, returns
/// `None` to prevent tool calls from interfering with voting.
pub fn new_for_vector(
    ensemble_llm_output_mode: objectiveai::ensemble_llm::OutputMode,
    request_tools: Option<&[objectiveai::chat::completions::request::Tool]>,
) -> Option<objectiveai::chat::completions::request::ToolChoice> {
    if let objectiveai::ensemble_llm::OutputMode::ToolCall = ensemble_llm_output_mode {
        Some(vector::completions::ResponseKey::tool_choice())
    } else if request_tools.is_some_and(|request_tools| !request_tools.is_empty()) {
        Some(objectiveai::chat::completions::request::ToolChoice::None)
    } else {
        None
    }
}
