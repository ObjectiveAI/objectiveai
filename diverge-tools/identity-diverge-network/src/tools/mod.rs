//! The two tools: one that lists, one that logs in.
//!
//! [`list_accounts`] lists the accounts the tool holds, and [`login`]
//! signs a subject of one in on the daemon's host.

mod list_accounts;
mod login;

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::failure::Failure;
use crate::server::Call;

/// Every tool's name, in the order they are listed.
pub const NAMES: [&str; 2] = [list_accounts::NAME, login::NAME];

/// Every tool, as `tools/list` states them.
pub fn all() -> Vec<Tool> {
    vec![tool::<list_accounts::Parameters>(list_accounts::NAME, list_accounts::DESCRIPTION), tool::<login::Parameters>(login::NAME, login::DESCRIPTION)]
}

/// Call the tool `name` with `arguments`; its structured answer.
pub async fn call(name: &str, call: Call<'_>, arguments: JsonObject) -> Result<Value, Failure> {
    match name {
        list_accounts::NAME => list_accounts::call(call, parameters(arguments)?).await,
        login::NAME => login::call(call, parameters(arguments)?).await,
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
