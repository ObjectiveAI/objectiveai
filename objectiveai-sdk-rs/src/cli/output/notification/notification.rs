use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wrapper that nests the notification payload one level deeper under a
/// `value` field. Required because [`super::super::Output`] uses
/// `#[serde(tag = "type")]` — if we let `T` flatten directly into the
/// outer JSON object, any `T` carrying its own `"type"` field (e.g. a
/// JSON Schema document with `"type": "object"`) would collide with
/// the discriminator.
///
/// `agent_id` is stamped at emit time by [`super::super::Handle`] when
/// its `agent_id` field is set; producers building a `Notification`
/// inline almost always leave the field `None` and let the handle
/// fill it. Serde-skipped when `None`.
///
/// Wire (in combination with `Output::Notification`):
/// `{"type":"notification","value":<T>,"agent_id":"<id>"?}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Notification.{T}")]
pub struct Notification<T> {
    pub value: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_id: Option<String>,
}
