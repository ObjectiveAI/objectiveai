//! Streaming agent completion chunk type.

use crate::agent::completions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A chunk of a streaming agent completion response.
///
/// Multiple chunks are received via Server-Sent Events and can be
/// accumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)
/// using the [`push`](Self::push) method.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "agent.completions.response.streaming.AgentCompletionChunk"
)]
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
    /// `true` when the MCP proxy holds queued messages that were not
    /// delivered to the agent via a tool response on this turn. Only
    /// set when `continuation` is also set — the caller acts on it by
    /// issuing the continuation. Absent when nothing is queued, when
    /// there is no continuation to act on, or when the peek failed
    /// (the failure is surfaced via `error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}

impl AgentCompletionChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
    pub fn push(
        &mut self,
        AgentCompletionChunk {
            messages,
            usage,
            error,
            continuation,
            messages_queued,
            ..
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
        if let Some(mq) = messages_queued {
            self.messages_queued = Some(*mq);
        }
    }

    /// Produces the [`LogFile`]s for the log file structure.
    ///
    /// Returns `None` if the chunk has no ID yet. All paths are relative
    /// to the `logs/` root directory, under `agents/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> Option<(
        crate::filesystem::logs::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    )> {
        use crate::filesystem::logs::{LogFile, LogReference};
        const ROUTE: &str = "agents/completions/response";

        let id = &self.id;
        if id.is_empty() {
            return None;
        }

        let mut files: Vec<LogFile> = Vec::new();
        let mut message_refs: Vec<LogReference> = Vec::new();

        for msg in &self.messages {
            let (reference, msg_files) = msg.produce_files(id, ROUTE);
            message_refs.push(reference);
            files.extend(msg_files);
        }

        // Extract continuation to a separate file (if present).
        let continuation_ref = self.continuation.as_ref().map(|continuation| {
            let cont_file = LogFile {
                route: format!("{ROUTE}/continuation"),
                id: id.clone(),
                message_index: None,
                media_index: None,
                extension: "txt".to_string(),
                content: continuation.clone().into_bytes(),
            };
            let r = LogReference::new(cont_file.path());
            files.push(cont_file);
            r
        });

        let log = super::AgentCompletionChunkLog {
            id: self.id.clone(),
            created: self.created,
            messages: message_refs,
            object: self.object,
            usage: self.usage.clone(),
            upstream: self.upstream,
            error: self.error.clone(),
            continuation: continuation_ref,
            messages_queued: self.messages_queued,
        };

        let root_file = LogFile {
            route: ROUTE.to_string(),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log).unwrap(),
        };
        let reference = LogReference::new(root_file.path());
        files.push(root_file);

        Some((reference, files))
    }

    /// Yields one [`MessageRow`] per `MessageChunk` for the SQLite
    /// `messages` table. Lazy: borrows from `self`, never collects.
    ///
    /// `agent_instance_hierarchy` is this chunk's `id`; `path` points at the
    /// per-message log file under `agents/completions/response/messages/`.
    /// Returns an empty iterator when `id` is empty (the chunk hasn't
    /// been assigned a response id yet — same gate `produce_files`
    /// uses).
    ///
    /// [`MessageRow`]: crate::filesystem::db::schema::MessageRow
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_
    {
        use crate::filesystem::db::schema::{MessageKind, MessageRow};
        let id = self.id.as_str();
        let created = self.created;
        let empty = self.id.is_empty();
        self.messages.iter().filter_map(move |m| {
            if empty {
                return None;
            }
            let kind = match m {
                super::MessageChunk::Assistant(_) => {
                    MessageKind::AssistantResponse
                }
                super::MessageChunk::Tool(_) => MessageKind::ToolResponse,
            };
            let idx = m.index();
            Some(MessageRow {
                agent_instance_hierarchy: id.to_string(),
                // Same value as agent_instance_hierarchy at this stage — the writer
                // will lineage-stamp `agent_instance_hierarchy` but `response_id`
                // stays bare so the reader doesn't have to parse it
                // back out of a stamped string.
                response_id: id.to_string(),
                kind,
                index: idx,
                // Bare id — the route is reconstructed from
                // (kind, response_id, path) by `MessageKind::file_path`.
                path: format!("{idx}"),
                timestamp: created,
            })
        })
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
