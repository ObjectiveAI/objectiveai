//! `delete_task`: take a task the caller posted off the board.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "delete_task";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Delete a task you posted, in any state, with its comments.";

/// What `delete_task` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
}

/// Delete the task.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    store::tasks::delete(pool, parameters.task, &call.caller).await?;
    Ok(json!({ "deleted": parameters.task }))
}
