//! The five tools: two that send, three that list.
//!
//! [`start`] opens a conversation and [`reply`] answers inside one;
//! each notifies the other agent and nothing more. [`conversations`]
//! lists the caller's conversations and [`messages`] reads one; each
//! shows unread only unless asked for everything, and reading marks
//! what was shown as read. [`agents`] lists whom the caller can write
//! to and be read by.

mod agents;
mod conversations;
mod messages;
mod reply;
mod send;
mod start;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::courier::Call;
use crate::failure::Failure;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 5] = [agents::NAME, start::NAME, reply::NAME, conversations::NAME, messages::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<agents::Parameters>(agents::NAME, agents::DESCRIPTION),
        tool::<start::Parameters>(start::NAME, start::DESCRIPTION),
        tool::<reply::Parameters>(reply::NAME, reply::DESCRIPTION),
        tool::<conversations::Parameters>(conversations::NAME, conversations::DESCRIPTION),
        tool::<messages::Parameters>(messages::NAME, messages::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        agents::NAME => agents::call(call, parameters(arguments)?).await,
        start::NAME => start::call(call, parameters(arguments)?).await,
        reply::NAME => reply::call(call, parameters(arguments)?).await,
        conversations::NAME => conversations::call(call, parameters(arguments)?).await,
        messages::NAME => messages::call(call, parameters(arguments)?).await,
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
