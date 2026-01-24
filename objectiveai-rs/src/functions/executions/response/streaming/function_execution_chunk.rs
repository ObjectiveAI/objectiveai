use crate::{error, functions, vector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionChunk {
    pub id: String,
    pub tasks: Vec<super::TaskChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks_errors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<super::ReasoningSummaryChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<functions::expression::FunctionOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_token: Option<String>,
    pub created: u64,
    pub function: Option<String>,
    pub profile: Option<String>,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<vector::completions::response::Usage>,
}

impl FunctionExecutionChunk {
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
            .is_some_and(vector::completions::response::Usage::any_usage)
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
}
