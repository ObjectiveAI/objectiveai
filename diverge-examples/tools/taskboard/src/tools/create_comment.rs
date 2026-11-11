//! `create_comment`: add to a task's discussion.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "create_comment";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Comment on a task, in any state. Answers the comment's id.";

/// What `create_comment` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The task, by the id `create_task` or `list_tasks` gave.
    pub task: i64,
    /// The comment.
    pub text: String,
}

/// Add the comment.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let id = store::comments::create(pool, parameters.task, &call.caller, &parameters.text).await?;
    Ok(json!({ "comment": id }))
}
