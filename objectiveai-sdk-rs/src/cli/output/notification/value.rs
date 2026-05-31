use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Generic wire wrapper used by every typed config getter:
/// `{"type":"notification","value":<T>}`. The element type varies
/// per config family (e.g. `Value<Option<String>>`, `Value<Option<u16>>`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Value.{T}")]
pub struct Value<T> {
    pub value: T,
}
