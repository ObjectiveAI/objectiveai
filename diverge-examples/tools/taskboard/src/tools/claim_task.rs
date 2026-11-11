//! `claim_task`: take an open task.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "claim_task";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Claim an open task; you hold it until you complete or release it. No two agents can hold the same task.";

/// What `claim_task` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
}

/// Claim the task.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    store::tasks::claim(pool, parameters.task, &call.caller).await?;
    Ok(json!({ "claimed": parameters.task }))
}
