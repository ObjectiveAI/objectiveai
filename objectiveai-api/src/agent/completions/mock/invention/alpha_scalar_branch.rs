use std::collections::HashMap;
use rand::Rng;
use super::super::client::{MockToolCall, random_string};
use crate::agent::completions::ResolvedTool;

/// Generate a mock tool call for the tasks step of a scalar branch function.
///
/// Scalar branch tasks are `PlaceholderScalarFunction` task expressions with
/// `name`, `spec`, `input_schema` (ObjectInputSchema), `input` (expression),
/// and optional `skip`.
///
/// The `input_schema_json` is the serialized parent `ScalarFunctionInputSchema`
/// obtained by calling `ReadInputSchema`. It can be ANY valid ObjectInputSchema
/// — arbitrary depth, types, constraints. The child schemas and input
/// expressions are derived from the actual parent schema.
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
            let field_schemas = super::extract_input_field_schemas(input_schema_json);
            let task = random_placeholder_scalar_task(&field_schemas, rng);
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

/// Generate a random placeholder scalar function task expression.
///
/// The child's `input_schema` uses the actual types from the parent schema,
/// and the `input` expression passes through matching fields.
fn random_placeholder_scalar_task(
    parent_fields: &[(String, serde_json::Value)],
    rng: &mut impl Rng,
) -> serde_json::Value {
    let spec = random_string(rng, 50, 200);

    // Child input schema: pick a subset of parent fields or pass through all
    let use_subset = parent_fields.len() > 1 && rng.random_range(0u32..2) == 0;
    let child_fields: Vec<&(String, serde_json::Value)> = if use_subset {
        let n = rng.random_range(1..parent_fields.len());
        parent_fields.iter().take(n).collect()
    } else {
        parent_fields.iter().collect()
    };

    // Build properties using actual parent types
    let mut properties = serde_json::Map::new();
    for (name, schema) in &child_fields {
        properties.insert(name.clone(), schema.clone());
    }
    let required: Vec<&str> = child_fields.iter().map(|(n, _)| n.as_str()).collect();

    // Input expression: pass through matching fields from parent
    let input_expr = if use_subset {
        // Build a dict literal with just the subset fields
        let entries: Vec<String> = child_fields.iter()
            .map(|(name, _)| format!("'{name}': input['{name}']"))
            .collect();
        format!("{{{}}}", entries.join(", "))
    } else {
        "input".to_string()
    };

    serde_json::json!({
        "type": "placeholder.alpha.scalar.function",
        "spec": spec,
        "input_schema": {
            "type": "object",
            "properties": properties,
            "required": required,
        },
        "input": { "$starlark": input_expr },
    })
}
