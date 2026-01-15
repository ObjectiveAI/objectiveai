use crate::{
    functions::{self, profiles::computations::response},
    vector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionProfileComputationChunk {
    pub id: String,
    pub executions: Vec<super::FunctionExecutionChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executions_errors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<functions::InlineProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fitting_stats: Option<response::FittingStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_token: Option<String>,
    pub created: u64,
    pub function: Option<String>,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<vector::completions::response::Usage>,
}

impl FunctionProfileComputationChunk {
    pub fn any_usage(&self) -> bool {
        self.usage
            .as_ref()
            .is_some_and(vector::completions::response::Usage::any_usage)
    }

    pub fn push(
        &mut self,
        FunctionProfileComputationChunk {
            executions,
            executions_errors,
            profile,
            fitting_stats,
            retry_token,
            usage,
            ..
        }: &FunctionProfileComputationChunk,
    ) {
        self.push_executions(executions);
        if let Some(true) = executions_errors {
            self.executions_errors = Some(true);
        }
        if let Some(profile) = profile {
            self.profile = Some(profile.clone());
        }
        if let Some(fitting_stats) = fitting_stats {
            self.fitting_stats = Some(fitting_stats.clone());
        }
        if let Some(retry_token) = retry_token {
            self.retry_token = Some(retry_token.clone());
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

    fn push_executions(
        &mut self,
        other_executions: &[super::FunctionExecutionChunk],
    ) {
        fn push_execution(
            executions: &mut Vec<super::FunctionExecutionChunk>,
            other: &super::FunctionExecutionChunk,
        ) {
            fn find_execution(
                executions: &mut Vec<super::FunctionExecutionChunk>,
                index: u64,
            ) -> Option<&mut super::FunctionExecutionChunk> {
                for execution in executions {
                    if execution.index == index {
                        return Some(execution);
                    }
                }
                None
            }
            if let Some(execution) = find_execution(executions, other.index) {
                execution.push(other);
            } else {
                executions.push(other.clone());
            }
        }
        for other_execution in other_executions {
            push_execution(&mut self.executions, other_execution);
        }
    }
}
