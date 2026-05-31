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
    /// [`LogReference`] pointing to this message's file, and `files`
    /// contains all produced [`LogFile`]s including the message itself
    /// and extracted media.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (crate::filesystem::logs::LogReference, Vec<crate::filesystem::logs::LogFile>) {
        use crate::filesystem::logs::{LogFile, LogReference};

        let mut files = Vec::new();

        // Extract media from content (flattened on disk via the
        // wire chunk's `serde(flatten)` on `inner`). Routed under the
        // kind-specific subdir so the (response_id, index) stems don't
        // collide with an assistant message at the same index.
        let mut content = self.inner.content.clone();
        content.prepare();
        let (content_log, media_files) = content.extract_media(
            &format!("{route_base}/messages/tool"),
            id,
            self.index,
        );
        files.extend(media_files);

        let log = super::ToolResponseLog {
            role: self.role,
            index: self.index,
            content: content_log,
            tool_call_id: self.inner.tool_call_id.clone(),
            metadata: self.inner.metadata.clone(),
        };

        let msg_file = LogFile {
            // Kind-specific subdir so this file can't collide with an
            // assistant message at the same (response_id, index) —
            // see `MessageKind::file_path` for the reader-side mirror.
            route: format!("{route_base}/messages/tool"),
            id: id.to_string(),
            message_index: Some(self.index),
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log).unwrap(),
        };
        let reference = LogReference::new(msg_file.path());
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
