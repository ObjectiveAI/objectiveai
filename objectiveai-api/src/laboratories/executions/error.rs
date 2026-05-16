/// Errors that can occur during laboratory execution.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Orchestrator operation failed.
    #[error("orchestrator error: {}", serde_json::to_string(.0).unwrap_or_default())]
    Orchestrator(objectiveai_sdk::error::ResponseError),
    /// MCP communication error.
    #[error("mcp error: {0}")]
    Mcp(String),
    /// No builder agents provided.
    #[error("at least one builder agent is required")]
    NoBuilderAgents,
    /// Agent completion failed.
    #[error("agent completion error: {0}")]
    AgentCompletion(String),
    /// Evaluation output failed to parse as JSON.
    #[error("evaluation output parse error: {0}")]
    EvaluationOutputParse(String),
    /// Evaluation output does not match the expected schema.
    #[error("evaluation output does not match schema")]
    EvaluationOutputSchemaMismatch,
    /// evaluation_agent and evaluation_output_schema must both be provided or both be omitted.
    #[error("evaluation_agent and evaluation_output_schema must both be provided or both be omitted")]
    EvaluationConfigMismatch,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::Orchestrator(_) => 500,
            Error::Mcp(_) => 500,
            Error::NoBuilderAgents => 400,
            Error::AgentCompletion(_) => 502,
            Error::EvaluationOutputParse(_) => 400,
            Error::EvaluationOutputSchemaMismatch => 400,
            Error::EvaluationConfigMismatch => 400,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "laboratory",
            "error": match self {
                Error::Orchestrator(e) => serde_json::json!({
                    "kind": "orchestrator",
                    "error": e.message,
                }),
                Error::Mcp(msg) => serde_json::json!({
                    "kind": "mcp",
                    "error": msg,
                }),
                Error::NoBuilderAgents => serde_json::json!({
                    "kind": "no_builder_agents",
                    "error": "at least one builder agent is required",
                }),
                Error::AgentCompletion(msg) => serde_json::json!({
                    "kind": "agent_completion",
                    "error": msg,
                }),
                Error::EvaluationOutputParse(msg) => serde_json::json!({
                    "kind": "evaluation_output_parse",
                    "error": msg,
                }),
                Error::EvaluationOutputSchemaMismatch => serde_json::json!({
                    "kind": "evaluation_output_schema_mismatch",
                    "error": "evaluation output does not match schema",
                }),
                Error::EvaluationConfigMismatch => serde_json::json!({
                    "kind": "evaluation_config_mismatch",
                    "error": "evaluation_agent and evaluation_output_schema must both be provided or both be omitted",
                }),
            }
        }))
    }
}
