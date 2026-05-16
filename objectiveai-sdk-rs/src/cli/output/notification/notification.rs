use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wrapper that nests the notification payload one level deeper under a
/// `value` field. Required because [`super::super::Output`] uses
/// `#[serde(tag = "type")]` — if we let `T` flatten directly into the
/// outer JSON object, any `T` carrying its own `"type"` field (e.g. a
/// JSON Schema document with `"type": "object"`) would collide with
/// the discriminator.
///
/// Wire (in combination with `Output::Notification`):
/// `{"type":"notification","value":<T>}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Notification.{T}")]
pub struct Notification<T> {
    pub value: T,
}
