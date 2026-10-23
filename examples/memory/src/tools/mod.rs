//! The three tools: one that keeps, one that finds, one that deletes.
//!
//! [`remember`] keeps a note, [`recall`] finds notes by a search of
//! their text and by their tags, and [`forget`] deletes one.

mod forget;
mod recall;
mod remember;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::failure::Failure;
use crate::memory::State;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 3] = [remember::NAME, recall::NAME, forget::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<remember::Parameters>(remember::NAME, remember::DESCRIPTION),
        tool::<recall::Parameters>(recall::NAME, recall::DESCRIPTION),
        tool::<forget::Parameters>(forget::NAME, forget::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, state: &State, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        remember::NAME => remember::call(state, parameters(arguments)?).await,
        recall::NAME => recall::call(state, parameters(arguments)?).await,
        forget::NAME => forget::call(state, parameters(arguments)?).await,
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
