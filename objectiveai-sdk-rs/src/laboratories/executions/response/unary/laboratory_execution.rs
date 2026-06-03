use crate::{agent, error, laboratories::executions::response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A complete laboratory execution response (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "laboratories.executions.response.unary.LaboratoryExecution"
)]
pub struct LaboratoryExecution {
    /// Unique identifier for this execution.
    pub id: String,
    /// Results from each builder agent completion.
    pub builders: Vec<super::Builder>,
    /// Results from each evaluation agent completion.
    pub evaluations: Vec<super::Evaluation>,
    /// Error details if the execution failed.
    pub error: Option<error::ResponseError>,
    /// Unix timestamp when the execution was created.
    pub created: u64,
    /// Object type identifier.
    pub object: super::Object,
    /// Aggregated token and cost usage.
    pub usage: agent::completions::response::Usage,
}

impl LaboratoryExecution {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
    }

    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.created = 0;
        for builder in &mut self.builders {
            builder.inner.normalize_for_tests();
            normalize_error(&mut builder.inner.error);
        }
        for evaluation in &mut self.evaluations {
            evaluation.inner.normalize_for_tests();
            normalize_error(&mut evaluation.inner.error);
        }
        // Sort by agent_index so two parallel builder/evaluation streams
        // settle into a stable order regardless of which chunk arrives
        // first off the wire. The chunk-level `index` field reflects
        // arrival order and is renumbered to match the sorted position.
        self.builders.sort_by_key(|b| b.agent_index);
        for (i, b) in self.builders.iter_mut().enumerate() {
            b.index = i as u64;
        }
        self.evaluations.sort_by_key(|e| e.agent_index);
        for (i, e) in self.evaluations.iter_mut().enumerate() {
            e.index = i as u64;
        }
    }
}

/// Replace dynamic port numbers in error messages with a placeholder.
fn normalize_error(error: &mut Option<crate::error::ResponseError>) {
    if let Some(e) = error {
        if let serde_json::Value::String(s) = &mut e.message {
            // Replace localhost:<port> with localhost:0
            while let Some(start) = s.find("localhost:") {
                let after = start + "localhost:".len();
                let end = s[after..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map(|i| after + i)
                    .unwrap_or(s.len());
                if end > after {
                    s.replace_range(after..end, "0");
                } else {
                    break;
                }
            }
        }
    }
}

impl From<response::streaming::LaboratoryExecutionChunk>
    for LaboratoryExecution
{
    fn from(
        response::streaming::LaboratoryExecutionChunk {
            id,
            builders,
            evaluations,
            error,
            created,
            object,
            usage,
        }: response::streaming::LaboratoryExecutionChunk,
    ) -> Self {
        Self {
            id,
            builders: builders.into_iter().map(super::Builder::from).collect(),
            evaluations: evaluations
                .into_iter()
                .map(super::Evaluation::from)
                .collect(),
            error,
            created,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
