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

/// Validates tool results in the continuation following this state.
///
/// Finds tool calls from this state that are `AppendTask` or
/// `WriteInputSchema` (with optional ` (objectiveai-invention)` suffix),
/// then checks the corresponding `ToolMessage` results. AppendTask must
/// return a digit string; WriteInputSchema must return `"Ok"`.
pub fn validate_continuation<'a>(
    state: &AssistantMessage,
    items_after: impl Iterator<Item = &'a super::super::ContinuationItem<AssistantMessage>>,
) -> Result<(), super::Error> {
    use objectiveai_sdk::agent::completions::message::RichContent;

    let calls = tool_calls_from_state(state);

    // Quick check: does this state have any watched tool calls?
    if !calls.iter().any(|(name, _)| is_append_task(name) || is_write_input_schema(name)) {
        return Ok(());
    }

    for item in items_after {
        if let super::super::ContinuationItem::ToolMessage(msg) = item {
            if let Some((name, _)) = calls.iter().find(|(_, id)| *id == msg.tool_call_id) {
                let content = match &msg.content {
                    RichContent::Text(t) => t.as_str(),
                    RichContent::Parts(_) => "",
                };
                if is_append_task(name) {
                    if content.is_empty() || !content.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(super::Error::AppendTaskFailed(content.to_string()));
                    }
                } else if is_write_input_schema(name) {
                    if content != "Ok" {
                        return Err(super::Error::WriteInputSchemaFailed(content.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}

const APPEND_TASK: &str = "AppendTask";
const APPEND_TASK_SUFFIXED: &str = "AppendTask (objectiveai-invention)";
const WRITE_INPUT_SCHEMA: &str = "WriteInputSchema";
const WRITE_INPUT_SCHEMA_SUFFIXED: &str = "WriteInputSchema (objectiveai-invention)";

fn is_append_task(name: &str) -> bool {
    name == APPEND_TASK || name == APPEND_TASK_SUFFIXED
}

fn is_write_input_schema(name: &str) -> bool {
    name == WRITE_INPUT_SCHEMA || name == WRITE_INPUT_SCHEMA_SUFFIXED
}
