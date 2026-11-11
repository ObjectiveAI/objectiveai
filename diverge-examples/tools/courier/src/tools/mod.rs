//! The five tools: two that send, three that list.
//!
//! [`create_conversation`] opens a conversation and [`reply`] answers
//! inside one; each notifies the other agent and nothing more.
//! [`list_conversations`] lists the caller's conversations and
//! [`list_messages`] reads one; each shows unread only unless asked
//! for everything, and reading marks what was shown as read.
//! [`list_agents`] lists whom the caller can write to and be read by.

mod create_conversation;
mod list_agents;
mod list_conversations;
mod list_messages;
mod reply;
mod send;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::courier::Call;
use crate::failure::Failure;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 5] = [list_agents::NAME, create_conversation::NAME, reply::NAME, list_conversations::NAME, list_messages::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<list_agents::Parameters>(list_agents::NAME, list_agents::DESCRIPTION),
        tool::<create_conversation::Parameters>(create_conversation::NAME, create_conversation::DESCRIPTION),
        tool::<reply::Parameters>(reply::NAME, reply::DESCRIPTION),
        tool::<list_conversations::Parameters>(list_conversations::NAME, list_conversations::DESCRIPTION),
        tool::<list_messages::Parameters>(list_messages::NAME, list_messages::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        list_agents::NAME => list_agents::call(call, parameters(arguments)?).await,
        create_conversation::NAME => create_conversation::call(call, parameters(arguments)?).await,
        reply::NAME => reply::call(call, parameters(arguments)?).await,
        list_conversations::NAME => list_conversations::call(call, parameters(arguments)?).await,
        list_messages::NAME => list_messages::call(call, parameters(arguments)?).await,
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
