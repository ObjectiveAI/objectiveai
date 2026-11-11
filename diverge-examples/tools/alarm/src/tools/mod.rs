//! The three tools: one that sets, one that lists, one that cancels.
//!
//! [`set_alarm`] sets an alarm for the caller, [`list_alarms`] shows
//! the caller's, and [`cancel_alarm`] cancels one. Every alarm wakes
//! the agent that set it; no tool reaches another agent's.

mod cancel_alarm;
mod list_alarms;
mod set_alarm;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::alarm::Call;
use crate::failure::Failure;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 3] = [set_alarm::NAME, list_alarms::NAME, cancel_alarm::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![
        tool::<set_alarm::Parameters>(set_alarm::NAME, set_alarm::DESCRIPTION),
        tool::<list_alarms::Parameters>(list_alarms::NAME, list_alarms::DESCRIPTION),
        tool::<cancel_alarm::Parameters>(cancel_alarm::NAME, cancel_alarm::DESCRIPTION),
    ]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        set_alarm::NAME => set_alarm::call(call, parameters(arguments)?).await,
        list_alarms::NAME => list_alarms::call(call, parameters(arguments)?).await,
        cancel_alarm::NAME => cancel_alarm::call(call, parameters(arguments)?).await,
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
