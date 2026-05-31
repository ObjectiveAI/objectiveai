use crate::{agent, error};
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Streaming chunk for a laboratory execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "laboratories.executions.response.streaming.LaboratoryExecutionChunk")]
pub struct LaboratoryExecutionChunk {
    pub id: String,
    pub builders: Vec<super::BuilderChunk>,
    pub evaluations: Vec<super::EvaluationChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}

impl AgentCompletionIds for LaboratoryExecutionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.builders
            .iter()
            .flat_map(|b| b.agent_completion_ids())
            .chain(self.evaluations.iter().flat_map(|e| e.agent_completion_ids()))
    }
}

impl LaboratoryExecutionChunk {
    /// Yields each inner error from this chunk's builders and evaluations,
    /// tagged with `(index, agent_index)` and discriminated by an
    /// [`InnerError`](super::InnerError) variant (`Builder` | `Evaluation`).
    ///
    /// Builder errors yield first (in vec order), then evaluation errors.
    /// Lazy and zero-allocation; collect with `.collect::<Vec<_>>()` if
    /// you need to retain the items past the chunk's lifetime.
    ///
    /// Does NOT include the chunk's own top-level `.error` field.
    pub fn inner_errors(&self) -> impl Iterator<Item = super::InnerError<'_>> {
        let builders = self.builders.iter().filter_map(|b| {
            b.inner.error.as_ref().map(|error| super::InnerError::Builder {
                builder_index: b.index,
                agent_completion_index: b.agent_index,
                error: std::borrow::Cow::Borrowed(error),
            })
        });
        let evaluations = self.evaluations.iter().filter_map(|e| {
            e.inner.error.as_ref().map(|error| super::InnerError::Evaluation {
                evaluation_index: e.index,
                agent_completion_index: e.agent_index,
                error: std::borrow::Cow::Borrowed(error),
            })
        });
        builders.chain(evaluations)
    }

    pub fn push(
        &mut self,
        LaboratoryExecutionChunk {
            builders,
            evaluations,
            error,
            usage,
            ..
        }: &LaboratoryExecutionChunk,
    ) {
        self.push_builders(builders);
        self.push_evaluations(evaluations);
        if let Some(error) = error {
            self.error = Some(error.clone());
        }
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
    }

    fn push_builders(&mut self, others: &[super::BuilderChunk]) {
        for other in others {
            if let Some(existing) = self.builders.iter_mut().find(|c| c.index == other.index) {
                existing.push(other);
            } else {
                self.builders.push(other.clone());
            }
        }
    }

    fn push_evaluations(&mut self, others: &[super::EvaluationChunk]) {
        for other in others {
            if let Some(existing) = self
                .evaluations
                .iter_mut()
                .find(|c| c.index == other.index)
            {
                existing.push(other);
            } else {
                self.evaluations.push(other.clone());
            }
        }
    }

}
