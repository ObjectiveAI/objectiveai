//! The eight tools: six over tasks, two over their comments.
//!
//! [`create_task`] posts a task and [`list_tasks`] shows the board.
//! [`claim_task`] takes an open task; [`complete_task`] and
//! [`release_task`] are its holder's alone, and [`delete_task`] its
//! poster's alone. [`create_comment`] and [`list_comments`] add to and
//! read a task's discussion.

mod claim_task;
mod complete_task;
mod create_comment;
mod create_task;
mod delete_task;
mod list_comments;
mod list_tasks;
mod release_task;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::failure::Failure;
use crate::taskboard::Call;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 8] = [create_task::NAME, list_tasks::NAME, claim_task::NAME, complete_task::NAME, release_task::NAME, delete_task::NAME, create_comment::NAME, list_comments::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<create_task::Parameters>(create_task::NAME, create_task::DESCRIPTION),
        tool::<list_tasks::Parameters>(list_tasks::NAME, list_tasks::DESCRIPTION),
        tool::<claim_task::Parameters>(claim_task::NAME, claim_task::DESCRIPTION),
        tool::<complete_task::Parameters>(complete_task::NAME, complete_task::DESCRIPTION),
        tool::<release_task::Parameters>(release_task::NAME, release_task::DESCRIPTION),
        tool::<delete_task::Parameters>(delete_task::NAME, delete_task::DESCRIPTION),
        tool::<create_comment::Parameters>(create_comment::NAME, create_comment::DESCRIPTION),
        tool::<list_comments::Parameters>(list_comments::NAME, list_comments::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        create_task::NAME => create_task::call(call, parameters(arguments)?).await,
        list_tasks::NAME => list_tasks::call(call, parameters(arguments)?).await,
        claim_task::NAME => claim_task::call(call, parameters(arguments)?).await,
        complete_task::NAME => complete_task::call(call, parameters(arguments)?).await,
        release_task::NAME => release_task::call(call, parameters(arguments)?).await,
        delete_task::NAME => delete_task::call(call, parameters(arguments)?).await,
        create_comment::NAME => create_comment::call(call, parameters(arguments)?).await,
        list_comments::NAME => list_comments::call(call, parameters(arguments)?).await,
        _ => Err(Failure::Parameters(format!("no tool named {name}"))),
    }
}

/// A tool, its input schema the JSON Schema of its parameters.
fn tool<P: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
    let schema = serde_json::to_value(schemars::schema_for!(P)).ok().and_then(|value| value.as_object().cloned()).unwrap_or_default();
    Tool::new(name, description, Arc::new(schema))
}

/// The call's arguments, read as the tool's parameters.
fn parameters<P: DeserializeOwned>(arguments: JsonObject) -> Result<P, Failure> {
    serde_json::from_value(Value::Object(arguments)).map_err(|error| Failure::Parameters(error.to_string()))
}
