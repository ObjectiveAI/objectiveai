use std::collections::HashMap;
use rand::Rng;
use super::super::client::{MockToolCall, random_string};
use crate::agent::completions::ResolvedTool;

/// Generate a mock tool call for the tasks step of a scalar leaf function.
///
/// Scalar leaf tasks are `VectorCompletion` task expressions with `messages`
/// (a Starlark expression) and `responses` (an array of text content parts).
///
/// The `input_schema_json` is the serialized `ScalarFunctionInputSchema`
/// obtained by calling the `ReadInputSchema` invention tool. It can be ANY
/// valid ObjectInputSchema — arbitrary depth, arbitrary types, enums, etc.
/// The generated expressions must correctly reference the schema's fields.
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
            let modalities = super::parse_scalar_schema(input_schema_json);
            let messages_expr = super::build_messages_expr("input", &modalities);
            let n_responses = rng.random_range(2u32..=5) as usize;
            let responses: Vec<serde_json::Value> = (0..n_responses)
                .map(|_| {
                    serde_json::json!([{"type": "text", "text": random_string(rng, 5, 40)}])
                })
                .collect();
            let task_json = serde_json::json!({
                "type": "vector.completion",
                "messages": { "$starlark": messages_expr },
                "responses": responses,
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
