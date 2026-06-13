use crate::{
    agent,
    functions::{self, profiles::computations::response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.profiles.computations.response.unary.FunctionProfileComputation"
)]
pub struct FunctionProfileComputation {
    pub id: String,
    pub executions: Vec<super::FunctionExecution>,
    pub executions_errors: bool,
    pub profile: functions::InlineTasksProfile,
    pub fitting_stats: response::FittingStats,
    pub created: u64,
    pub function: Option<crate::RemotePath>,
    pub object: super::Object,
    pub usage: agent::completions::response::Usage,
}

impl FunctionProfileComputation {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
    }

    pub fn normalize_for_tests(&mut self) {}
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
            profile: profile.unwrap_or_else(|| functions::InlineTasksProfile {
                tasks: Vec::new(),
                weights: None,
            }),
            fitting_stats: fitting_stats
                .unwrap_or(response::FittingStats::default()),
            created,
            function,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
