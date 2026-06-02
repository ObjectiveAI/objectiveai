use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::{agent, error, functions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.inventions.response.streaming.FunctionInventionChunk"
)]
pub struct FunctionInventionChunk {
    pub id: String,
    pub completions: Vec<super::AgentCompletionChunk>,
    // yielded after steps with the current state
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub state: Option<functions::inventions::State>,
    // yielded at the end
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub path: Option<crate::RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub function: Option<functions::FullRemoteFunction>,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}

impl AgentCompletionIds for FunctionInventionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.completions
            .iter()
            .flat_map(|c| c.agent_completion_ids())
    }
}

impl FunctionInventionChunk {
}

impl FunctionInventionChunk {
    /// Yields each inner error from this chunk's per-agent completions,
    /// tagged with the failing completion's `index`.
    ///
    /// Lazy and zero-allocation; collect with `.collect::<Vec<_>>()` if you
    /// need to retain the items past the chunk's lifetime.
    ///
    /// Does NOT include the chunk's own top-level `.error` field.
    pub fn inner_errors(&self) -> impl Iterator<Item = super::InnerError<'_>> {
        self.completions.iter().filter_map(|c| {
            c.inner.error.as_ref().map(|error| super::InnerError {
                agent_completion_index: c.index,
                error: std::borrow::Cow::Borrowed(error),
            })
        })
    }

    pub fn push(
        &mut self,
        FunctionInventionChunk {
            completions,
            state,
            path,
            function,
            usage,
            error,
            ..
        }: &FunctionInventionChunk,
    ) {
        self.push_completions(completions);
        if let Some(state) = state {
            self.state = Some(state.clone());
        }
        if let Some(path) = path {
            self.path = Some(path.clone());
        }
        if let Some(function) = function {
            self.function = Some(function.clone());
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
        if let Some(error) = error {
            self.error = Some(error.clone());
        }
    }

    fn push_completions(
        &mut self,
        other_completions: &[super::AgentCompletionChunk],
    ) {
        fn push_completion(
            completions: &mut Vec<super::AgentCompletionChunk>,
            other: &super::AgentCompletionChunk,
        ) {
            fn find_completion(
                completions: &mut Vec<super::AgentCompletionChunk>,
                index: u64,
            ) -> Option<&mut super::AgentCompletionChunk> {
                for completion in completions {
                    if completion.index == index {
                        return Some(completion);
                    }
                }
                None
            }
            if let Some(completion) = find_completion(completions, other.index)
            {
                completion.push(other);
            } else {
                completions.push(other.clone());
            }
        }
        for other_completion in other_completions {
            push_completion(&mut self.completions, other_completion);
        }
    }

}
