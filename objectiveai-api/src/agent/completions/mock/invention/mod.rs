pub(super) mod alpha_scalar;
pub(super) mod alpha_scalar_branch;
pub(super) mod alpha_scalar_leaf;
pub(super) mod alpha_vector;
pub(super) mod alpha_vector_branch;
pub(super) mod alpha_vector_leaf;
mod route;
pub(super) mod schema_gen;

pub use route::*;

/// Pick an invention tool with weighted selection.
///
/// 50% chance: return the key write tool for this step.
/// 50% chance: pick randomly from the available tool names. Invention
/// tools now flow through the proxy as `ResolvedTool::Mcp`, so callers
/// must ensure `tool_names` only contains invention-relevant tools for
/// the test scenario at hand.
pub(super) fn pick_invention_tool<'a>(
    key_tool: &str,
    tool_names: &'a [String],
    _tool_map: &std::collections::HashMap<String, crate::agent::completions::ResolvedTool>,
    rng: &mut impl rand::Rng,
) -> &'a str {
    // 50% chance: return the key tool
    if rng.random_range(0u32..2) == 0 {
        if let Some(t) = tool_names.iter().find(|t| t.as_str() == key_tool) {
            return t.as_str();
        }
    }
    // 50% chance: pick randomly from all available tool names.
    let names: Vec<&'a str> = tool_names.iter().map(|t| t.as_str()).collect();
    names[rng.random_range(0..names.len())]
}

/// Generate a mock tool call for the description step (shared across all routes).
///
/// Tools: `[ReadSpec, ReadEssay, ReadInputSchema, ReadEssayTasks, ReadTask,
/// ReadTasksLength, WriteDescription]`.
pub fn description_tool_call(
    tool_names: &[String],
    tool_map: &std::collections::HashMap<String, crate::agent::completions::ResolvedTool>,
    rng: &mut impl rand::Rng,
) -> super::client::MockToolCall {
    use super::client::{MockToolCall, random_string};

    let tool_name = pick_invention_tool("WriteDescription", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "WriteDescription" => {
            let description = random_string(rng, 50, 350);
            serde_json::json!({ "description": description }).to_string()
        }
        "ReadSpec" | "ReadEssay" | "ReadInputSchema" | "ReadEssayTasks"
        | "ReadTasksLength" => "{}".to_string(),
        "ReadTask" => {
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

// ---------------------------------------------------------------------------
// Static multimodal content parts for Starlark expressions.
//
// Leaf tasks include static content parts matching the modalities declared
// in the relevant portion of the input schema. This guarantees modality
// coverage (AS20/AV18) while only emitting the parts that the validator
// actually expects.
// ---------------------------------------------------------------------------

use objectiveai::functions::expression::Modalities;

const STATIC_IMAGE: &str =
    r#"{"type": "image_url", "image_url": {"url": "https://example.com/test.png"}}"#;
const STATIC_AUDIO: &str =
    r#"{"type": "input_audio", "input_audio": {"data": "dGVzdA==", "format": "wav"}}"#;
const STATIC_VIDEO: &str =
    r#"{"type": "video_url", "video_url": {"url": "https://example.com/test.mp4"}}"#;
const STATIC_FILE: &str =
    r#"{"type": "file", "file": {"file_data": "dGVzdA=="}}"#;

/// Collect static multimodal part strings matching the given modalities.
fn static_media_parts(m: &Modalities) -> Vec<&'static str> {
    let mut parts = Vec::new();
    if m.image { parts.push(STATIC_IMAGE); }
    if m.audio { parts.push(STATIC_AUDIO); }
    if m.video { parts.push(STATIC_VIDEO); }
    if m.file { parts.push(STATIC_FILE); }
    parts
}

/// Build a Starlark messages expression for a vector completion task.
///
/// Stringifies the entire input (or sub-path) as a text content part and
/// includes static multimodal content parts matching the given modalities.
fn build_messages_expr(input_expr: &str, modalities: &Modalities) -> String {
    let text_part = format!(
        r#"{{"type": "text", "text": str({input_expr})}}"#,
    );
    let media = static_media_parts(modalities);
    let all_parts: Vec<&str> = std::iter::once(text_part.as_str()).chain(media).collect();
    let content = all_parts.join(", ");
    format!(r#"[{{"role": "user", "content": [{content}]}}]"#)
}

/// Build a Starlark responses expression for a vector leaf function.
///
/// Stringifies each item as a text content part and includes static
/// multimodal content parts matching the given modalities.
fn build_responses_expr(modalities: &Modalities) -> String {
    let text_part = r#"{"type": "text", "text": str(item)}"#;
    let media = static_media_parts(modalities);
    let all_parts: Vec<&str> = std::iter::once(text_part).chain(media).collect();
    let content = all_parts.join(", ");
    format!("[[{content}] for item in input['items']]")
}

/// Parse a VectorFunctionInputSchema from JSON and return (context_modalities, items_modalities, has_context).
fn parse_vector_schema(input_schema_json: &str) -> (Modalities, Modalities, bool) {
    use objectiveai::functions::alpha_vector::expression::VectorFunctionInputSchema;
    match serde_json::from_str::<VectorFunctionInputSchema>(input_schema_json) {
        Ok(schema) => {
            let ctx_mod = schema.context.as_ref()
                .map(|c| c.modalities())
                .unwrap_or_default();
            let items_mod = schema.items.modalities();
            let has_ctx = schema.context.is_some();
            (ctx_mod, items_mod, has_ctx)
        }
        Err(_) => (Modalities::default(), Modalities::default(), false),
    }
}

/// Parse a ScalarFunctionInputSchema (ObjectInputSchema) from JSON and return its modalities.
fn parse_scalar_schema(input_schema_json: &str) -> Modalities {
    use objectiveai::functions::expression::ObjectInputSchema;
    serde_json::from_str::<ObjectInputSchema>(input_schema_json)
        .map(|s| s.modalities())
        .unwrap_or_default()
}

/// Extract required field names with their full JSON schemas from a serialized
/// `ObjectInputSchema` JSON. Returns `(name, schema)` pairs.
///
/// Used by branch task generators to build child placeholder schemas with
/// the actual parent field types.
fn extract_input_field_schemas(input_schema_json: &str) -> Vec<(String, serde_json::Value)> {
    serde_json::from_str::<serde_json::Value>(input_schema_json)
        .ok()
        .and_then(|v| {
            let props = v.get("properties")?.as_object()?;
            // Prefer required fields
            let names: Vec<String> = v.get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_else(|| props.keys().cloned().collect());
            let result: Vec<(String, serde_json::Value)> = names.into_iter()
                .map(|name| {
                    let schema = props.get(&name).cloned()
                        .unwrap_or_else(|| serde_json::json!({"type": "string"}));
                    (name, schema)
                })
                .collect();
            if result.is_empty() { None } else { Some(result) }
        })
        .unwrap_or_else(|| vec![("text".to_string(), serde_json::json!({"type": "string"}))])
}

/// Extract the task count range (min, max) from a tasks step prompt.
///
/// Looks for "between N and M" or "Create N " patterns.
pub fn extract_task_count_range(prompt: &str) -> (u32, u32) {
    // Try "between N and M"
    if let Some(idx) = prompt.find("between ") {
        let rest = &prompt[idx + 8..];
        let parts: Vec<&str> = rest.splitn(4, ' ').collect();
        if parts.len() >= 3 && parts[1] == "and" {
            if let (Ok(min), Ok(max)) = (parts[0].parse::<u32>(), parts[2].parse::<u32>()) {
                return (min, max);
            }
        }
    }
    // Try "Create N " — find the first occurrence where N is a digit.
    let mut search_from = 0;
    while let Some(idx) = prompt[search_from..].find("Create ") {
        let abs_idx = search_from + idx;
        let rest = &prompt[abs_idx + 7..];
        if let Some(n_str) = rest.split_whitespace().next() {
            if let Ok(n) = n_str.parse::<u32>() {
                return (n, n);
            }
        }
        search_from = abs_idx + 7;
    }
    // Fallback
    (3, 5)
}
