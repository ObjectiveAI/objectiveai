use std::collections::HashMap;
use rand::Rng;
use super::super::client::MockToolCall;
use crate::agent::completions::ResolvedTool;

/// Generate a mock tool call for the tasks step of a vector leaf function.
///
/// Vector leaf tasks are `VectorCompletion` task expressions with `messages`
/// (a Starlark expression) and `responses` (a Starlark expression producing
/// an array derived from input items).
///
/// The `input_schema_json` is the serialized `VectorFunctionInputSchema`
/// obtained by calling `ReadInputSchema`. It can be ANY valid vector input
/// schema — items of any type, arbitrary item object depth, optional context
/// with any structure, etc.
pub fn tasks_tool_call(
    input_schema_json: &str,
    tasks_min: u64,
    tool_names: &[String],
    tool_map: &HashMap<String, ResolvedTool>,
    rng: &mut impl Rng,
) -> MockToolCall {
    let tool_name = super::pick_invention_tool("oaifi_AppendTask", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "oaifi_AppendTask" => {
            let (ctx_modalities, items_modalities, has_context) =
                super::parse_vector_schema(input_schema_json);

            // Messages: use context modalities if context exists, otherwise static
            let messages_expr = if has_context {
                super::build_messages_expr("input['context']", &ctx_modalities)
            } else {
                super::build_messages_expr("input", &objectiveai_sdk::functions::expression::Modalities::default())
            };

            // Responses: use items modalities
            let responses_expr = super::build_responses_expr(&items_modalities);

            let task_json = serde_json::json!({
                "type": "vector.completion",
                "messages": { "$starlark": messages_expr },
                "responses": { "$starlark": responses_expr },
            }).to_string();
            serde_json::json!({"task": task_json}).to_string()
        }
        "oaifi_EditPredictedTasksLength" => {
            serde_json::json!({"tasks_length": tasks_min}).to_string()
        }
        "oaifi_DeleteTask" | "oaifi_ReadTask" => {
            serde_json::json!({ "index": rng.random_range(0u32..5) }).to_string()
        }
        _ => "{}".to_string(),
    };
    MockToolCall {
        tool_name: tool_name.to_string(),
        call_id: format!("call_mock_{}", rng.random_range(0u64..u64::MAX)),
        arguments,
        n_deltas: rng.random_range(1u32..=5) as usize,
    }
}
