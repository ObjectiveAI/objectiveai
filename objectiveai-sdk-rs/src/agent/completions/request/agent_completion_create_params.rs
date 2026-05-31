//! Agent completion request parameters.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Parameters for creating a agent completion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.request.AgentCompletionCreateParams")]
pub struct AgentCompletionCreateParams {
    /// The conversation messages.
    pub messages: Vec<super::super::message::Message>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<super::Provider>,
    /// The agent to use (inline Agent or stored ID).
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    /// Output format constraints (text, JSON, or JSON schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_format: Option<super::ResponseFormatParam>,
    /// Random seed for deterministic generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

#[cfg(feature = "filesystem")]
impl crate::filesystem::logs::ProducesRequestFiles for AgentCompletionCreateParams {
    /// Break this request out into per-leaf log files:
    /// - each `message` → its own JSON file holding a
    ///   [`super::super::message::MessageLog`] (with content extracted
    ///   to further per-leaf files alongside).
    /// - `response_format` (if Some) → its own JSON file.
    /// - `continuation` (if Some) → its own .txt file.
    /// - The top-level
    ///   [`super::AgentCompletionCreateParamsLog`] gets written as
    ///   `<route_base>/<id>.json`; its `LogReference` is returned
    ///   along with every produced [`crate::filesystem::logs::LogFile`].
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (
        crate::filesystem::logs::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        use crate::filesystem::logs::{LogFile, LogReference};

        let mut all_files: Vec<LogFile> = Vec::new();

        // 1. messages — each becomes its own MessageLog JSON.
        let mut message_refs = Vec::with_capacity(self.messages.len());
        for (msg_idx, message) in self.messages.iter().cloned().enumerate() {
            let (msg_log, content_files) = message.extract(
                route_base,
                id,
                msg_idx as u64,
            );
            all_files.extend(content_files);
            let msg_file = LogFile {
                route: format!("{route_base}/messages"),
                id: id.to_string(),
                message_index: Some(msg_idx as u64),
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(&msg_log)
                    .expect("MessageLog serializes"),
            };
            message_refs.push(LogReference::new(msg_file.path()));
            all_files.push(msg_file);
        }

        // 2. response_format → own file.
        let response_format_ref = self.response_format.as_ref().map(|rf| {
            let file = LogFile {
                route: format!("{route_base}/response_format"),
                id: id.to_string(),
                message_index: None,
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(rf)
                    .expect("ResponseFormatParam serializes"),
            };
            let r = LogReference::new(file.path());
            all_files.push(file);
            r
        });

        // 3. continuation → own file (raw bytes — it's a base64 string).
        let continuation_ref = self.continuation.as_ref().map(|c| {
            let file = LogFile {
                route: format!("{route_base}/continuation"),
                id: id.to_string(),
                message_index: None,
                media_index: None,
                extension: "txt".to_string(),
                content: c.clone().into_bytes(),
            };
            let r = LogReference::new(file.path());
            all_files.push(file);
            r
        });

        // 4. The top-level Log envelope.
        let log = super::AgentCompletionCreateParamsLog {
            messages: message_refs,
            provider: self.provider.clone(),
            agent: self.agent.clone(),
            response_format: response_format_ref,
            seed: self.seed,
            stream: self.stream,
            continuation: continuation_ref,
        };
        let summary_file = LogFile {
            route: route_base.to_string(),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log)
                .expect("AgentCompletionCreateParamsLog serializes"),
        };
        let summary_ref = LogReference::new(summary_file.path());
        all_files.push(summary_file);

        (summary_ref, all_files)
    }
}
