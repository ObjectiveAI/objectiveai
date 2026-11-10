//! `complete_task`: finish a task the caller holds.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "complete_task";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Complete a task you hold, with its result.";

/// What `complete_task` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
    /// What came of it.
    pub result: String,
}

/// Complete the task.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    store::tasks::complete(pool, parameters.task, &call.caller, &parameters.result).await?;
    Ok(json!({ "completed": parameters.task }))
}
