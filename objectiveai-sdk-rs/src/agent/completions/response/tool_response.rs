use crate::agent;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.response.ToolResponse")]
pub struct ToolResponse {
    pub role: ToolRole,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::message::ToolMessage,
}

impl ToolResponse {
    /// Produces log files for this tool message.
    ///
    /// Returns `(reference, files)` where `reference` is a
    /// `{"type": "reference", "path": ...}` JSON value, and `files`
    /// contains all produced [`LogFile`]s including the message itself and
    /// extracted media.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self, id: &str, route_base: &str) -> (serde_json::Value, Vec<crate::filesystem::logs::LogFile>) {
        use crate::filesystem::logs::LogFile;

        let mut files = Vec::new();

        // Serialize a shell without content to avoid double-serialization
        let shell = ToolResponse {
            role: self.role,
            index: self.index,
            inner: agent::completions::message::ToolMessage {
                content: agent::completions::message::RichContent::Text(String::new()),
                tool_call_id: self.inner.tool_call_id.clone(),
            },
        };
        let mut msg_json = serde_json::to_value(&shell).unwrap();

        // Extract media from content (flattened, so "content" is at root)
        let mut content = self.inner.content.clone();
        content.prepare();
        let (content_json, media_files) = content.extract_media(route_base, id, self.index);
        msg_json["content"] = content_json;
        files.extend(media_files);

        let msg_file = LogFile {
            route: format!("{route_base}/messages"),
            id: id.to_string(),
            message_index: Some(self.index),
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&msg_json).unwrap(),
        };
        let reference = serde_json::json!({ "type": "reference", "path": msg_file.path() });
        files.push(msg_file);

        (reference, files)
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.ToolRole")]
pub enum ToolRole {
    #[serde(rename = "tool")]
    #[default]
    Tool,
}
