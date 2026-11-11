//! The two tools: the agent reads the person's messages, and answers.
//!
//! [`read_messages`] hands the agent what the person wrote — unread
//! only, or everything — and marks it read; [`send_message`] keeps the
//! agent's answer and shows it in every open page at once.

mod read_messages;
mod send_message;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::chat::Call;
use crate::failure::Failure;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 2] = [read_messages::NAME, send_message::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<read_messages::Parameters>(read_messages::NAME, read_messages::DESCRIPTION),
        tool::<send_message::Parameters>(send_message::NAME, send_message::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        read_messages::NAME => read_messages::call(call, parameters(arguments)?).await,
        send_message::NAME => send_message::call(call, parameters(arguments)?).await,
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
