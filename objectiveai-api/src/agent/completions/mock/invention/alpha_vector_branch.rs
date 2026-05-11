use std::collections::HashMap;
use rand::Rng;
use super::super::client::{MockToolCall, random_string};
use crate::agent::completions::ResolvedTool;

/// Generate a mock tool call for the tasks step of a vector branch function.
///
/// Vector branch tasks can be either:
/// - `placeholder.alpha.vector.function` — ranks items relative to each other
/// - `placeholder.alpha.scalar.function` — scores individual items (max 50% of tasks)
///
/// The `input_schema_json` is the serialized `VectorFunctionInputSchema`
/// obtained by calling `ReadInputSchema`. It can be ANY valid vector input
/// schema — items of any type, arbitrary depth, optional context, etc.
///
/// `scalar_count` and `total_count` track how many scalar tasks have been
/// appended so far, ensuring at most 50% are scalar.
pub fn tasks_tool_call(
    input_schema_json: &str,
    scalar_count: u32,
    total_count: u32,
    tasks_min: u64,
    tool_names: &[String],
    tool_map: &HashMap<String, ResolvedTool>,
    rng: &mut impl Rng,
) -> MockToolCall {
    let tool_name = super::pick_invention_tool("oaifi_AppendTask", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "oaifi_AppendTask" => {
            // Parse the full input schema to extract items and context sub-schemas
            let parsed = serde_json::from_str::<serde_json::Value>(input_schema_json)
                .unwrap_or_else(|_| serde_json::json!({}));

            // Decide scalar vs vector. Scalar is only allowed if:
            // - There's already at least one vector task (a branch with a single
            //   scalar task is invalid per AW08).
            // - Under 50% of tasks would be scalar after adding this one.
            let has_vector = total_count > scalar_count;
            let scalar_allowed = has_vector
                && (scalar_count as f64 / (total_count + 1) as f64) < 0.5;
            let use_scalar = scalar_allowed && rng.random_range(0u32..3) == 0; // ~33% chance

            let task = if use_scalar {
                random_placeholder_scalar_task(&parsed, rng)
            } else {
                random_placeholder_vector_task(&parsed, rng)
            };
            serde_json::json!({"task": task.to_string()}).to_string()
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

/// Generate a random placeholder vector function task expression.
///
/// The child vector function receives the parent's items (and optionally
/// context). The child's input_schema mirrors the parent's structure with
/// the actual types preserved.
fn random_placeholder_vector_task(
    parent_schema: &serde_json::Value,
    rng: &mut impl Rng,
) -> serde_json::Value {
    let spec = random_string(rng, 50, 200);

    // Build child input_schema from parent's items and context.
    // parent_schema["items"] is the per-element schema (transpile wraps in array).
    let mut child_schema = serde_json::Map::new();

    // Items: copy per-element schema from parent directly
    let items = parent_schema.get("items").cloned().unwrap_or_else(|| {
        serde_json::json!({"type": "string"})
    });
    child_schema.insert("items".into(), items);

    // Context: copy from parent if present
    let mut input_expr = serde_json::Map::new();
    input_expr.insert("items".into(), serde_json::json!({"$starlark": "input['items']"}));

    if let Some(context) = parent_schema.get("context") {
        child_schema.insert("context".into(), context.clone());
        input_expr.insert("context".into(), serde_json::json!({"$starlark": "input['context']"}));
    }

    serde_json::json!({
        "type": "placeholder.alpha.vector.function",
        "spec": spec,
        "input_schema": child_schema,
        "input": input_expr,
    })
}

/// Generate a random placeholder scalar function task expression.
///
/// Scalar sub-functions in a vector branch are mapped over items (transpile
/// sets `map: InputItemsOutputLength`). The `map` variable is an index, so
/// the input expression indexes into `input['items']` using `map`.
///
/// The child's input_schema uses actual types from the parent's per-element
/// schema. `parent_schema["items"]` is the per-element schema directly
/// (transpile wraps it in an array).
fn random_placeholder_scalar_task(
    parent_schema: &serde_json::Value,
    rng: &mut impl Rng,
) -> serde_json::Value {
    let spec = random_string(rng, 50, 200);

    // parent_schema["items"] is the per-element schema
    let item_schema = parent_schema.get("items");

    let (child_schema, input_expr) = if let Some(item_s) = item_schema {
        if item_s.get("type").and_then(|t| t.as_str()) == Some("object") {
            // Items are objects — use the item schema as the child schema
            let child = item_s.clone();
            (child, "input['items'][map]".to_string())
        } else {
            // Items are simple types (string, image, etc.) — wrap in an object
            let item_type_schema = item_s.clone();
            (serde_json::json!({
                "type": "object",
                "properties": {"value": item_type_schema},
                "required": ["value"],
            }), "{'value': input['items'][map]}".to_string())
        }
    } else {
        // Fallback
        (serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "string"}},
            "required": ["value"],
        }), "{'value': str(input['items'][map])}".to_string())
    };

    serde_json::json!({
        "type": "placeholder.alpha.scalar.function",
        "spec": spec,
        "input_schema": child_schema,
        "input": { "$starlark": input_expr },
    })
}
