use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::NotificationValue;

/// Wrapper that nests the notification payload one level deeper under a
/// `value` field. Required because [`super::super::Output`] uses
/// `#[serde(tag = "type")]` — keeping `NotificationValue` under `value`
/// instead of flattening preserves the historical `.value.<field>`
/// access path for downstream consumers.
///
/// `agent_id` is stamped at emit time by [`super::super::Handle`] when
/// its `agent_id` field is set; producers building a `Notification`
/// inline almost always leave the field `None` and let the handle
/// fill it. Serde-skipped when `None`.
///
/// Wire (in combination with `Output::Notification`):
/// `{"type":"notification","value":{"kind":"<variant>",…},"agent_id":"<id>"?}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Notification")]
pub struct Notification {
    pub value: NotificationValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_id: Option<String>,
}
