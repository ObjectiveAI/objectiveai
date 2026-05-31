use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Synchronous response from `objectiveai viewer send` — pairs the
/// HTTP status the viewer returned with the parsed JSON body.
///
/// Wire: `{"type":"notification","value":{"kind":"viewer_send_result","status":200,"body":{…}}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.ViewerSendResult")]
pub struct ViewerSendResult {
    pub status: u16,
    pub body: serde_json::Value,
}
