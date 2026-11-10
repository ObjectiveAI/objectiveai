//! `list_tasks`: the board.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::store;
use crate::taskboard::Call;

/// The tool's name.
pub const NAME: &str = "list_tasks";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "List the board's tasks, oldest first: every task, or only those open, claimed or done. Each with its id, title, detail, \
state, who posted it, who holds it, its result, its times and how many comments it has.";

/// What `list_tasks` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Only tasks in this state: `open`, `claimed` or `done`.
    /// Absent, every task.
    #[serde(default)]
    pub state: Option<store::tasks::State>,
    /// At most this many tasks. Absent, 50.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// How many tasks a list shows when the caller does not say.
const LIMIT: u32 = 50;

/// The board.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    let pool = call.state.store.pool(&call.state.client).await?;
    let limit = i64::from(parameters.limit.unwrap_or(LIMIT));
    let tasks = store::tasks::list(pool, parameters.state, limit).await?;
    Ok(json!({ "tasks": tasks }))
}
