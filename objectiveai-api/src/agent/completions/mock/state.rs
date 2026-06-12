use objectiveai_sdk::agent::completions::message::AssistantMessage;

/// Extracts `(tool_name, tool_call_id)` pairs from an AssistantMessage's tool calls.
pub fn tool_calls_from_state(state: &AssistantMessage) -> Vec<(&str, &str)> {
    state.tool_calls.as_ref().map_or(Vec::new(), |tcs| {
        tcs.iter().map(|tc| match tc {
            objectiveai_sdk::agent::completions::message::AssistantToolCall::Function { id, function } => {
                (function.name.as_str(), id.as_str())
            }
        }).collect()
    })
}

/// Returns the number of tool calls in the state.
pub fn tool_call_count(state: &AssistantMessage) -> usize {
    state.tool_calls.as_ref().map_or(0, |tcs| tcs.len())
}
