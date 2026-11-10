//! `release_task`: give a held task back.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "release_task";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Release a task you hold, back to open for any agent to claim.";

/// What `release_task` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
}

/// Release the task.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    store::tasks::release(pool, parameters.task, &call.caller).await?;
    Ok(json!({ "released": parameters.task }))
}
