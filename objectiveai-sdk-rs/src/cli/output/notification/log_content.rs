use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Contents of a log read or subscribe. Mirrors the upstream
/// `crate::filesystem::logs::LogContent` (which has no serde
/// derives) so the wire shape is parseable in cli-lib without depending
/// on it for serialization.
///
/// `Json` carries an arbitrary API payload (one of the two intentional
/// uses of `serde_json::Value` in this module). `DataUrl` carries
/// `data:{mime};base64,{payload}` for binary log content.
///
/// Wire shapes:
///   `{"type":"notification","content":<value>}`
///   `{"type":"notification","content_data_url":"data:..."}`
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.output.notification.LogContent")]
pub enum LogContent {
    #[schemars(title = "Json")]
    Json { content: serde_json::Value },
    #[schemars(title = "DataUrl")]
    DataUrl { content_data_url: String },
}
