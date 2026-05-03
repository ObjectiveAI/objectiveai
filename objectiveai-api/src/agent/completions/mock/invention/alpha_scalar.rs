use std::collections::HashMap;
use rand::Rng;
use super::super::client::{MockToolCall, random_string};
use crate::agent::completions::ResolvedTool;


/// Generate a mock tool call for the essay step of a scalar function.
///
/// Picks a random tool from the available tools. If the chosen tool is an
/// invention tool (`WriteEssay`, `ReadSpec`), generates appropriate arguments.
/// Otherwise falls back to schema-based argument generation.
pub fn essay_tool_call(
    tool_names: &[String],
    tool_map: &HashMap<String, ResolvedTool>,
    rng: &mut impl Rng,
) -> MockToolCall {
    let tool_name = super::pick_invention_tool("objectiveai-function-invention_WriteEssay", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "objectiveai-function-invention_WriteEssay" => {
            let essay = random_string(rng, 200, 800);
            serde_json::json!({ "essay": essay }).to_string()
        }
        "objectiveai-function-invention_ReadSpec" => "{}".to_string(),
        _ => "{}".to_string(),
    };
    MockToolCall {
        tool_name: tool_name.to_string(),
        call_id: format!("call_mock_{}", rng.random_range(0u64..u64::MAX)),
        arguments,
        n_deltas: rng.random_range(1u32..=5) as usize,
    }
}

/// Generate a mock tool call for the input_schema step of a scalar function.
///
/// If the chosen tool is `WriteInputSchema`, generates a random
/// `ScalarFunctionInputSchema` with diverse property names/types.
pub fn input_schema_tool_call(
    tool_names: &[String],
    tool_map: &HashMap<String, ResolvedTool>,
    rng: &mut impl Rng,
) -> MockToolCall {
    let tool_name = super::pick_invention_tool("objectiveai-function-invention_WriteInputSchema", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "objectiveai-function-invention_WriteInputSchema" => {
            let schema_json = super::schema_gen::random_scalar_input_schema(rng);
            serde_json::json!({"schema": schema_json}).to_string()
        }
        "objectiveai-function-invention_ReadSpec" | "objectiveai-function-invention_ReadEssay" | "objectiveai-function-invention_ReadInputSchema" => "{}".to_string(),
        _ => "{}".to_string(),
    };
    MockToolCall {
        tool_name: tool_name.to_string(),
        call_id: format!("call_mock_{}", rng.random_range(0u64..u64::MAX)),
        arguments,
        n_deltas: rng.random_range(1u32..=5) as usize,
    }
}

/// Generate a mock tool call for the essay_tasks step of a scalar function.
///
/// If the chosen tool is `WriteEssayTasks`, generates a random essay tasks
/// string. Read tools get empty arguments. Other tools use schema-based
/// generation.
pub fn essay_tasks_tool_call(
    tool_names: &[String],
    tool_map: &HashMap<String, ResolvedTool>,
    rng: &mut impl Rng,
) -> MockToolCall {
    let tool_name = super::pick_invention_tool("objectiveai-function-invention_WriteEssayTasks", tool_names, tool_map, rng);
    let arguments = match tool_name {
        "objectiveai-function-invention_WriteEssayTasks" => {
            let essay_tasks = random_string(rng, 100, 500);
            serde_json::json!({ "essay_tasks": essay_tasks }).to_string()
        }
        "objectiveai-function-invention_ReadSpec" | "objectiveai-function-invention_ReadEssay" | "objectiveai-function-invention_ReadInputSchema" => "{}".to_string(),
        _ => "{}".to_string(),
    };
    MockToolCall {
        tool_name: tool_name.to_string(),
        call_id: format!("call_mock_{}", rng.random_range(0u64..u64::MAX)),
        arguments,
        n_deltas: rng.random_range(1u32..=5) as usize,
    }
}
