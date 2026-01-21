use crate::{
    functions::{self, profiles::computations::response},
    vector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionProfileComputation {
    pub id: String,
    pub executions: Vec<super::FunctionExecution>,
    pub executions_errors: bool,
    pub profile: functions::InlineProfile,
    pub fitting_stats: response::FittingStats,
    pub retry_token: Option<String>,
    pub created: u64,
    pub function: Option<String>,
    pub object: super::Object,
    pub usage: vector::completions::response::Usage,
}

impl FunctionProfileComputation {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
    }
}

impl From<response::streaming::FunctionProfileComputationChunk>
    for FunctionProfileComputation
{
    fn from(
        response::streaming::FunctionProfileComputationChunk {
            id,
            executions,
            executions_errors,
            profile,
            fitting_stats,
            retry_token,
            created,
            function,
            object,
            usage,
        }: response::streaming::FunctionProfileComputationChunk,
    ) -> Self {
        Self {
            id,
            executions: executions
                .into_iter()
                .map(super::FunctionExecution::from)
                .collect(),
            executions_errors: executions_errors.unwrap_or(false),
            profile: profile.unwrap_or_else(|| functions::InlineProfile {
                tasks: Vec::new(),
            }),
            fitting_stats: fitting_stats
                .unwrap_or(response::FittingStats::default()),
            retry_token,
            created,
            function,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
