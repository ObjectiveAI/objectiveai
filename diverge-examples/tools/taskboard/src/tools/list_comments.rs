//! `list_comments`: read a task's discussion.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "list_comments";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Read a task's comments, oldest first, each with who made it and when.";

/// What `list_comments` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
}

/// The task's comments.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let comments = store::comments::list(pool, parameters.task).await?;
    Ok(json!({ "comments": comments }))
}
