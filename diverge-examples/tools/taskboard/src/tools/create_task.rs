//! `create_task`: post a task to the board.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "create_task";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Post a task to the board, open for any agent to claim. Answers the task's id.";

/// What `create_task` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// What the task is, in a line.
    pub title: String,
    /// What the task asks, in full.
    pub detail: String,
}

/// Post the task.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let id = store::tasks::create(pool, &parameters.title, &parameters.detail, &call.caller).await?;
    Ok(json!({ "task": id }))
}
