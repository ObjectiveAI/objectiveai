use crate::{agent, error};
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.executions.response.streaming.FunctionExecutionChunk")]
pub struct FunctionExecutionChunk {
    pub id: String,
    pub tasks: Vec<super::TaskChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tasks_errors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::ReasoningSummaryChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub output: Option<super::super::Output>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<String>,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub function: Option<crate::RemotePath>,
    pub profile: Option<crate::RemotePath>,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}

impl AgentCompletionIds for FunctionExecutionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.tasks
            .iter()
            .flat_map(|t| t.agent_completion_ids())
            .chain(
                self.reasoning
                    .as_ref()
                    .into_iter()
                    .flat_map(|r| r.agent_completion_ids()),
            )
    }
}

impl FunctionExecutionChunk {
    /// Flat-maps message rows from every task (mirrors
    /// `agent_completion_ids()`'s traversal). Reasoning summary rows
    /// are also included via the reasoning chunk's delegation. Lazy
    /// and `Box<dyn Iterator>`-erased at this boundary because tasks
    /// and reasoning have different concrete iterator types.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> Box<dyn Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_> {
        let task_rows = self.tasks.iter().flat_map(|t| t.produce_message_rows());
        let reasoning_rows = self
            .reasoning
            .iter()
            .flat_map(|r| r.produce_message_rows());
        Box::new(task_rows.chain(reasoning_rows))
    }
}

impl FunctionExecutionChunk {
    /// Yields every inner error reachable from this chunk: nested function-task
    /// failures, vector-completion task failures (own + per-agent), and
    /// reasoning agent-completion failures. Each item carries the full
    /// hierarchical `task_path` to uniquely identify the failing site.
    ///
    /// Does NOT include the chunk's own top-level `.error` field, nor the
    /// reasoning summary's own `.error` (only the inner agent completion's
    /// failure surfaces as a reasoning error).
    ///
    /// Lazy: walks the task tree on demand. Recursion is type-erased via
    /// `Box<dyn Iterator>` at each nested level — one small heap allocation
    /// per traversed depth, not per error. The hot path avoids
    /// `ResponseError` clones via `Cow::Borrowed`.
    pub fn inner_errors(&self) -> impl Iterator<Item = super::InnerError<'_>> {
        self.inner_errors_at(&[])
    }

    fn inner_errors_at<'a>(
        &'a self,
        my_task_path: &'a [u64],
    ) -> Box<dyn Iterator<Item = super::InnerError<'a>> + Send + 'a> {
        let task_errors = self.tasks.iter().flat_map(
            |task| -> Box<dyn Iterator<Item = super::InnerError<'a>> + Send + 'a> {
                match task {
                    super::TaskChunk::FunctionExecution(wrapper) => {
                        let own = wrapper.inner.error.as_ref().map(|error| {
                            super::InnerError::FunctionTaskError {
                                task_path: wrapper.task_path.clone(),
                                swiss_pool_index: wrapper.swiss_pool_index,
                                swiss_round: wrapper.swiss_round,
                                split_index: wrapper.split_index,
                                error: std::borrow::Cow::Borrowed(error),
                            }
                        });
                        let nested = wrapper.inner.inner_errors_at(&wrapper.task_path);
                        Box::new(own.into_iter().chain(nested))
                    }
                    super::TaskChunk::VectorCompletion(wrapper) => {
                        let task_path = &wrapper.task_path;
                        let own = wrapper.error.as_ref().map(|error| {
                            super::InnerError::VectorCompletionTaskError {
                                task_path: task_path.clone(),
                                agent_completion_index: None,
                                error: std::borrow::Cow::Borrowed(error),
                            }
                        });
                        let agents = wrapper.inner.completions.iter().filter_map(move |c| {
                            c.inner.error.as_ref().map(|error| {
                                super::InnerError::VectorCompletionTaskError {
                                    task_path: task_path.clone(),
                                    agent_completion_index: Some(c.index),
                                    error: std::borrow::Cow::Borrowed(error),
                                }
                            })
                        });
                        Box::new(own.into_iter().chain(agents))
                    }
                }
            },
        );
        let reasoning_errors = self.reasoning.iter().filter_map(move |r| {
            // Intentionally skip r.error (the summary's own .error);
            // only the inner agent completion's failure surfaces here.
            r.inner.error.as_ref().map(|error| {
                super::InnerError::ReasoningAgentCompletionError {
                    task_path: my_task_path.to_vec(),
                    error: std::borrow::Cow::Borrowed(error),
                }
            })
        });
        Box::new(task_errors.chain(reasoning_errors))
    }

    pub fn vector_completion_tasks(
        &self,
    ) -> impl Iterator<Item = &super::VectorCompletionTaskChunk> {
        self.tasks
            .iter()
            .flat_map(|task| task.vector_completion_tasks())
    }

    pub fn any_usage(&self) -> bool {
        self.usage
            .as_ref()
            .is_some_and(agent::completions::response::Usage::any_usage)
    }

    pub fn push(
        &mut self,
        FunctionExecutionChunk {
            tasks,
            tasks_errors,
            reasoning,
            output,
            retry_token,
            error,
            usage,
            ..
        }: &FunctionExecutionChunk,
    ) {
        self.push_tasks(tasks);
        if let Some(true) = tasks_errors {
            self.tasks_errors = Some(true);
        }
        match (&mut self.reasoning, &reasoning) {
            (Some(self_reasoning), Some(other_reasoning)) => {
                self_reasoning.push(other_reasoning);
            }
            (None, Some(other_reasoning)) => {
                self.reasoning = Some(other_reasoning.clone());
            }
            _ => {}
        }
        if let Some(output) = output {
            self.output = Some(output.clone());
        }
        if let Some(retry_token) = retry_token {
            self.retry_token = Some(retry_token.clone());
        }
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

    fn push_tasks(&mut self, other_tasks: &[super::TaskChunk]) {
        fn push_task(
            tasks: &mut Vec<super::TaskChunk>,
            other: &super::TaskChunk,
        ) {
            fn find_task(
                tasks: &mut Vec<super::TaskChunk>,
                index: u64,
            ) -> Option<&mut super::TaskChunk> {
                for task in tasks {
                    if task.index() == index {
                        return Some(task);
                    }
                }
                None
            }
            if let Some(task) = find_task(tasks, other.index()) {
                task.push(other);
            } else {
                tasks.push(other.clone());
            }
        }
        for other_task in other_tasks {
            push_task(&mut self.tasks, other_task);
        }
    }

    /// Produces the [`LogFile`]s for the log file structure.
    ///
    /// Returns `(reference, files)`. All paths relative to `logs/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> Option<(crate::filesystem::logs::LogReference, Vec<crate::filesystem::logs::LogFile>)> {
        use crate::filesystem::logs::{LogFile, LogReference};
        const ROUTE: &str = "functions/executions/response";

        let id = &self.id;
        if id.is_empty() {
            return None;
        }

        let mut files: Vec<LogFile> = Vec::new();
        let mut task_refs: Vec<super::task_log_reference::LogReference> = Vec::new();

        for task in &self.tasks {
            let (reference, task_files) = task.produce_files();
            task_refs.push(reference);
            files.extend(task_files);
        }

        let reasoning_ref = self.reasoning.as_ref().map(|r| {
            let (reference, reasoning_files) = r.produce_files();
            files.extend(reasoning_files);
            reference
        });

        let retry_token_ref = self.retry_token.as_ref().map(|retry_token| {
            let rt_file = LogFile {
                route: format!("{ROUTE}/retry_token"),
                id: id.clone(),
                message_index: None,
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(retry_token).unwrap(),
            };
            let r = LogReference::new(rt_file.path());
            files.push(rt_file);
            r
        });

        let log = super::FunctionExecutionChunkLog {
            id: self.id.clone(),
            tasks: task_refs,
            tasks_errors: self.tasks_errors,
            output: self.output.clone(),
            error: self.error.clone(),
            retry_token: retry_token_ref,
            created: self.created,
            function: self.function.clone(),
            profile: self.profile.clone(),
            object: self.object,
            usage: self.usage.clone(),
            reasoning: reasoning_ref,
        };

        let root_file = LogFile {
            route: ROUTE.to_string(),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log).unwrap(),
        };
        let reference = LogReference::new(root_file.path());
        files.push(root_file);

        Some((reference, files))
    }
}
