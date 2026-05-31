//! Streaming agent completion chunk type.

use crate::agent::completions::{message, response};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A chunk of a streaming agent completion response.
///
/// Multiple chunks are received via Server-Sent Events and can be
/// accumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)
/// using the [`push`](Self::push) method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.response.streaming.AssistantResponseChunk")]
pub struct AssistantResponseChunk {
    pub role: response::AssistantRole,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub agent: String,
    pub model: String,
    pub upstream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<message::AssistantToolCallDelta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<message::RichContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<String>,
    pub finish_reason: Option<response::FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub logprobs: Option<response::Logprobs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<String>,
    /// Upstream usage for this assistant response (set by upstream clients).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::UpstreamUsage>,
}

impl AssistantResponseChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
    pub fn push(
        &mut self,
        AssistantResponseChunk {
            reasoning,
            tool_calls,
            content,
            refusal,
            finish_reason,
            logprobs,
            upstream_id,
            service_tier,
            system_fingerprint,
            provider,
            usage,
            ..
        }: &AssistantResponseChunk,
    ) {
        response::util::push_option_string(&mut self.reasoning, reasoning);
        self.push_tool_calls(tool_calls);
        match (&mut self.content, content) {
            (Some(self_content), Some(other_content)) => {
                self_content.push(other_content);
            }
            (None, Some(other_content)) => {
                self.content = Some(other_content.clone());
            }
            _ => {}
        }
        response::util::push_option_string(&mut self.refusal, refusal);
        if self.finish_reason.is_none() {
            self.finish_reason = finish_reason.clone();
        }
        match (&mut self.logprobs, logprobs) {
            (Some(self_logprobs), Some(other_logprobs)) => {
                self_logprobs.push(other_logprobs);
            }
            (None, Some(other_logprobs)) => {
                self.logprobs = Some(other_logprobs.clone());
            }
            _ => {}
        }
        if self.upstream_id.is_empty() {
            self.upstream_id = upstream_id.clone();
        }
        if self.service_tier.is_none() {
            self.service_tier = service_tier.clone();
        }
        if self.system_fingerprint.is_none() {
            self.system_fingerprint = system_fingerprint.clone();
        }
        if self.provider.is_none() {
            self.provider = provider.clone();
        }
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
    }

    fn push_tool_calls(
        &mut self,
        other_tool_calls: &Option<Vec<message::AssistantToolCallDelta>>,
    ) {
        fn push_tool_call(
            tool_calls: &mut Vec<message::AssistantToolCallDelta>,
            other: &message::AssistantToolCallDelta,
        ) {
            fn find_tool_call(
                tool_calls: &mut Vec<message::AssistantToolCallDelta>,
                index: u64,
            ) -> Option<&mut message::AssistantToolCallDelta> {
                for tool_call in tool_calls {
                    if tool_call.index == index {
                        return Some(tool_call);
                    }
                }
                None
            }
            if let Some(tool_call) = find_tool_call(tool_calls, other.index) {
                tool_call.push(other);
            } else {
                tool_calls.push(other.clone());
            }
        }
        match (self.tool_calls.as_mut(), other_tool_calls) {
            (Some(self_tool_calls), Some(other_tool_calls)) => {
                for other_tool_call in other_tool_calls {
                    push_tool_call(self_tool_calls, other_tool_call);
                }
            }
            (None, Some(other_tool_calls)) => {
                self.tool_calls = Some(other_tool_calls.clone());
            }
            _ => {}
        }
    }

    /// Produces log files for this assistant message.
    ///
    /// Returns `(reference, files)` where `reference` is a
    /// [`LogReference`] pointing to this message's file, and `files`
    /// contains all produced [`LogFile`]s including the message itself,
    /// logprobs, and extracted media.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (crate::filesystem::logs::LogReference, Vec<crate::filesystem::logs::LogFile>) {
        use crate::filesystem::logs::{LogFile, LogReference};

        let mut files = Vec::new();

        // All assistant-only extracts live under the kind subdir so
        // every reference from the parent assistant message log file
        // points strictly inside its own directory subtree — see the
        // nested-sub-folder rule on `LogReference`.

        // Extract logprobs to a separate file (if present).
        let logprobs_ref = self.logprobs.as_ref().map(|logprobs| {
            let logprobs_file = LogFile {
                route: format!("{route_base}/messages/assistant/logprobs"),
                id: id.to_string(),
                message_index: Some(self.index),
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(logprobs).unwrap(),
            };
            let r = LogReference::new(logprobs_file.path());
            files.push(logprobs_file);
            r
        });

        // Extract reasoning to its own file (if present).
        let reasoning_ref = self.reasoning.as_ref().map(|reasoning| {
            let f = LogFile {
                route: format!("{route_base}/messages/assistant/reasoning"),
                id: id.to_string(),
                message_index: Some(self.index),
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(reasoning).unwrap(),
            };
            let r = LogReference::new(f.path());
            files.push(f);
            r
        });

        // Extract refusal to its own file (if present).
        let refusal_ref = self.refusal.as_ref().map(|refusal| {
            let f = LogFile {
                route: format!("{route_base}/messages/assistant/refusal"),
                id: id.to_string(),
                message_index: Some(self.index),
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(refusal).unwrap(),
            };
            let r = LogReference::new(f.path());
            files.push(f);
            r
        });

        // Extract each tool_call to its own file (if present).
        let tool_call_refs = self.tool_calls.as_ref().map(|tcs| {
            tcs.iter()
                .map(|tc| {
                    let f = LogFile {
                        route: format!("{route_base}/messages/assistant/tool_calls"),
                        id: id.to_string(),
                        message_index: Some(self.index),
                        media_index: Some(tc.index),
                        extension: "json".to_string(),
                        content: serde_json::to_vec_pretty(tc).unwrap(),
                    };
                    let r = LogReference::new(f.path());
                    files.push(f);
                    r
                })
                .collect::<Vec<_>>()
        });

        // Extract media from content (if present). Routed under the
        // kind-specific subdir so the (response_id, index) stems don't
        // collide with a tool message at the same index.
        let content_log = self.content.clone().map(|mut content| {
            content.prepare();
            let (content_log, media_files) = content.extract_media(
                &format!("{route_base}/messages/assistant"),
                id,
                self.index,
            );
            files.extend(media_files);
            content_log
        });

        let log = super::AssistantResponseChunkLog {
            role: self.role,
            index: self.index,
            created: self.created,
            agent: self.agent.clone(),
            model: self.model.clone(),
            upstream_id: self.upstream_id.clone(),
            reasoning: reasoning_ref,
            tool_calls: tool_call_refs,
            content: content_log,
            refusal: refusal_ref,
            finish_reason: self.finish_reason.clone(),
            logprobs: logprobs_ref,
            service_tier: self.service_tier.clone(),
            system_fingerprint: self.system_fingerprint.clone(),
            provider: self.provider.clone(),
            usage: self.usage.clone(),
        };

        let msg_file = LogFile {
            // Kind-specific subdir so this file can't collide with a
            // tool message at the same (response_id, index) — see
            // `MessageKind::file_path` for the reader-side mirror.
            route: format!("{route_base}/messages/assistant"),
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
