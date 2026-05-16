//! Streaming agent completion chunk type.

use crate::agent::completions::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A chunk of a streaming agent completion response.
///
/// Multiple chunks are received via Server-Sent Events and can be
/// accumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)
/// using the [`push`](Self::push) method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.response.streaming.AgentCompletionChunk")]
pub struct AgentCompletionChunk {
    pub id: String,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub messages: Vec<super::MessageChunk>,
    /// The object type (always "agent.completion.chunk").
    pub object: super::Object,
    /// Token usage (only present in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::Usage>,
    /// Upstream provider
    pub upstream: crate::agent::Upstream,
    /// Error details if this completion failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<crate::error::ResponseError>,
    /// Continuation state for multi-turn conversations (only present in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

impl AgentCompletionChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
    pub fn push(
        &mut self,
        AgentCompletionChunk {
            messages, usage, error, continuation, ..
        }: &AgentCompletionChunk,
    ) {
        self.push_messages(messages);
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
        if let Some(error) = error {
            self.error = Some(error.clone());
        }
        if let Some(continuation) = continuation {
            self.continuation = Some(continuation.clone());
        }
    }

    /// Produces the [`LogFile`]s for the log file structure.
    ///
    /// Returns `None` if the chunk has no ID yet. All paths are relative
    /// to the `logs/` root directory, under `agents/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> Option<(serde_json::Value, Vec<crate::filesystem::logs::LogFile>)> {
        use crate::filesystem::logs::LogFile;
        const ROUTE: &str = "agents/completions";

        let id = &self.id;
        if id.is_empty() {
            return None;
        }

        let mut files: Vec<LogFile> = Vec::new();
        let mut message_refs: Vec<serde_json::Value> = Vec::new();

        for msg in &self.messages {
            let (reference, msg_files) = msg.produce_files(id, ROUTE);
            message_refs.push(reference);
            files.extend(msg_files);
        }

        // Serialize a shell without messages/continuation to avoid double-serialization
        let shell = AgentCompletionChunk {
            id: self.id.clone(),
            created: self.created,
            messages: Vec::new(),
            object: self.object,
            usage: self.usage.clone(),
            upstream: self.upstream,
            error: self.error.clone(),
            continuation: Some(String::new()),
        };
        let mut root = serde_json::to_value(&shell).unwrap();
        root["messages"] = serde_json::Value::Array(message_refs);

        // Extract continuation to a separate file, or remove placeholder
        if let Some(continuation) = &self.continuation {
            let cont_file = LogFile {
                route: format!("{ROUTE}/continuation"),
                id: id.clone(),
                message_index: None,
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(continuation).unwrap(),
            };
            root["continuation"] = serde_json::json!({
                "type": "reference",
                "path": cont_file.path(),
            });
            files.push(cont_file);
        } else if let Some(map) = root.as_object_mut() {
            map.remove("continuation");
        }

        let root_file = LogFile {
            route: ROUTE.to_string(),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&root).unwrap(),
        };
        let reference = serde_json::json!({ "type": "reference", "path": root_file.path() });
        files.push(root_file);

        Some((reference, files))
    }

    fn push_messages(&mut self, other_choices: &[super::MessageChunk]) {
        fn push_message(
            messages: &mut Vec<super::MessageChunk>,
            other: &super::MessageChunk,
        ) {
            fn find_message(
                messages: &mut Vec<super::MessageChunk>,
                index: u64,
            ) -> Option<&mut super::MessageChunk> {
                for message in messages {
                    if message.index() == index {
                        return Some(message);
                    }
                }
                None
            }
            if let Some(message) = find_message(messages, other.index()) {
                message.push(other);
            } else {
                messages.push(other.clone());
            }
        }
        for other_message in other_choices {
            push_message(&mut self.messages, other_message);
        }
    }
}
