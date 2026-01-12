use crate::{
    functions::{self, compute_profile::response},
    vector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComputeProfile {
    pub id: String,
    pub executions: Vec<super::FunctionExecution>,
    pub executions_errors: bool,
    pub profile: functions::ComputedProfile,
    pub fitting_stats: response::FittingStats,
    pub retry_token: Option<String>,
    pub created: u64,
    pub function: String,
    pub object: super::Object,
    pub usage: vector::completions::response::Usage,
}

impl FunctionComputeProfile {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
    }
}

impl From<response::streaming::FunctionComputeProfileChunk>
    for FunctionComputeProfile
{
    fn from(
        response::streaming::FunctionComputeProfileChunk {
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
        }: response::streaming::FunctionComputeProfileChunk,
    ) -> Self {
        Self {
            id,
            executions: executions
                .into_iter()
                .map(super::FunctionExecution::from)
                .collect(),
            executions_errors: executions_errors.unwrap_or(false),
            profile: profile.unwrap_or_else(|| functions::ComputedProfile {
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
