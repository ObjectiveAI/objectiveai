use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single JSON Schema document, emitted by `schemas <category> <type> get`.
/// The body is the raw schema as a `Value` since JSON Schemas have an
/// open recursive structure.
///
/// Wire: `{"type":"notification","schema":{...JSON Schema...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Schema")]
pub struct Schema {
    pub schema: serde_json::Value,
}

/// Names of the schemas at the current namespace level, emitted by
/// `schemas list` and per-category `schemas <category> list` commands.
///
/// Wire: `{"type":"notification","schemas":["name1","name2",...]}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Schemas")]
pub struct Schemas {
    pub schemas: Vec<String>,
}
