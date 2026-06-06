//! Typed classification of paths stored in the `files` table.
//!
//! Every distinct write site under `logs/` produces files in one of a
//! finite set of "kinds" — `read_agent_completion_message_assistant_image`'s
//! `agents/completions/response/messages/assistant/image/<id>_<msg>_<media>.<ext>`,
//! `read_agent_completion_message_assistant_reasoning`'s
//! `.../messages/assistant/reasoning/<id>_<msg>.txt`, and so on. Each
//! kind maps 1:1 to a typed `Client::read_*` method, and to the args
//! that method requires.
//!
//! [`LogFileKind::from_path`] parses a logs-relative path into the
//! matching variant. `Client::read_file_by_id` looks up the id's path,
//! classifies it via this enum, and dispatches to the matching
//! `read_*` method — so the JSON-vs-DataUrl-vs-Text decision lives in
//! exactly one place: the variant-to-method match arm.

use std::str::FromStr;

/// Classification of a logs-relative path. Each variant names a
/// concrete `Client::read_*` method and carries the args that method
/// needs.
///
/// Variants are ordered roughly the same as the `Client::read_*`
/// declarations: top-level envelopes first, then per-message
/// metadata, then per-message content (assistant + tool response +
/// request + notification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFileKind {
    // -- Top-level envelopes ------------------------------------------------
    /// `agents/completions/response/<id>.json`
    /// — [`crate::filesystem::Client::read_agent_completion`].
    AgentCompletion { id: String },
    /// `agents/completions/request/<id>.json`
    /// — [`crate::filesystem::Client::read_agent_completion_request`].
    AgentCompletionRequest { id: String },
    /// `agents/completions/response/continuation/<id>.txt`
    /// — [`crate::filesystem::Client::read_agent_completion_continuation`].
    AgentCompletionContinuation { id: String },
    /// `agents/completions/request/continuation/<id>.txt`
    /// — [`crate::filesystem::Client::read_agent_completion_request_continuation`].
    AgentCompletionRequestContinuation { id: String },
    /// `vector/completions/response/<id>.json`
    /// — [`crate::filesystem::Client::read_vector_completion`].
    VectorCompletion { id: String },
    /// `vector/completions/request/<id>.json`
    /// — [`crate::filesystem::Client::read_vector_completion_request`].
    VectorCompletionRequest { id: String },
    /// `functions/executions/response/<id>.json`
    /// — [`crate::filesystem::Client::read_function_execution`].
    FunctionExecution { id: String },
    /// `functions/executions/request/<id>.json`
    /// — [`crate::filesystem::Client::read_function_execution_request`].
    FunctionExecutionRequest { id: String },
    /// `functions/executions/response/retry_token/<id>.txt`
    /// — [`crate::filesystem::Client::read_function_execution_retry_token`].
    FunctionExecutionRetryToken { id: String },
    /// `functions/inventions/response/<id>.json`
    /// — [`crate::filesystem::Client::read_function_invention`].
    FunctionInvention { id: String },
    /// `functions/inventions/request/<id>.json`
    /// — [`crate::filesystem::Client::read_function_invention_request`].
    FunctionInventionRequest { id: String },
    /// `functions/inventions/recursive/response/<id>.json`
    /// — [`crate::filesystem::Client::read_function_invention_recursive`].
    FunctionInventionRecursive { id: String },
    /// `functions/inventions/recursive/request/<id>.json`
    /// — [`crate::filesystem::Client::read_function_invention_recursive_request`].
    FunctionInventionRecursiveRequest { id: String },

    // -- Per-message metadata (response side, JSON) -------------------------
    /// `agents/completions/response/messages/assistant/<id>_<msg>.json`
    /// — an `AssistantResponseChunkLog` envelope, exactly as written.
    /// [`crate::filesystem::Client::read_agent_completion_message_assistant`].
    AgentCompletionMessageAssistant { id: String, message_index: u64 },
    /// `agents/completions/response/messages/tool/<id>_<msg>.json`
    /// — a `ToolResponseLog` envelope, exactly as written.
    /// [`crate::filesystem::Client::read_agent_completion_message_tool`].
    AgentCompletionMessageTool { id: String, message_index: u64 },
    /// `agents/completions/response/messages/assistant/logprobs/<id>_<msg>.json`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_logprobs`].
    AgentCompletionMessageAssistantLogprobs { id: String, message_index: u64 },
    /// `agents/completions/response/messages/assistant/reasoning/<id>_<msg>.txt`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_reasoning`].
    AgentCompletionMessageAssistantReasoning { id: String, message_index: u64 },
    /// `agents/completions/response/messages/assistant/refusal/<id>_<msg>.txt`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_refusal`].
    AgentCompletionMessageAssistantRefusal { id: String, message_index: u64 },
    /// `agents/completions/response/messages/assistant/tool_calls/<id>_<msg>_<tc>.json`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_tool_call`].
    AgentCompletionMessageAssistantToolCall {
        id: String,
        message_index: u64,
        tool_call_index: u64,
    },

    // -- Assistant content (response side) ---------------------------------
    /// `agents/completions/response/messages/assistant/text/<id>_<msg>[_<part>].txt`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_text`].
    AgentCompletionMessageAssistantText {
        id: String,
        message_index: u64,
        media_index: Option<u64>,
    },
    /// `agents/completions/response/messages/assistant/image/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_image`].
    AgentCompletionMessageAssistantImage {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/assistant/audio/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_audio`].
    AgentCompletionMessageAssistantAudio {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/assistant/video/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_video`].
    AgentCompletionMessageAssistantVideo {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/assistant/file/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_assistant_file`].
    AgentCompletionMessageAssistantFile {
        id: String,
        message_index: u64,
        media_index: u64,
    },

    // -- Tool response content (response side, under .../messages/tool/) ---
    /// `agents/completions/response/messages/tool/text/<id>_<msg>[_<part>].txt`
    /// — [`crate::filesystem::Client::read_agent_completion_message_tool_text`].
    AgentCompletionMessageToolText {
        id: String,
        message_index: u64,
        media_index: Option<u64>,
    },
    /// `agents/completions/response/messages/tool/image/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_tool_image`].
    AgentCompletionMessageToolImage {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/tool/audio/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_tool_audio`].
    AgentCompletionMessageToolAudio {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/tool/video/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_tool_video`].
    AgentCompletionMessageToolVideo {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/response/messages/tool/file/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_message_tool_file`].
    AgentCompletionMessageToolFile {
        id: String,
        message_index: u64,
        media_index: u64,
    },

    // -- Request-side message content --------------------------------------
    // Request messages share one bare `messages/` namespace for every
    // role (system / user / developer / tool / assistant) — message
    // indices are unique across roles within a request, so there is no
    // role-subdir split like the response side has.
    /// `agents/completions/request/messages/<id>_<msg>.json` — the
    /// per-role `MessageLog` envelope, any role.
    /// — [`crate::filesystem::Client::read_agent_completion_request_message`].
    AgentCompletionRequestMessage { id: String, message_index: u64 },
    /// `agents/completions/request/messages/text/<id>_<msg>[_<part>].txt`
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_text`].
    AgentCompletionRequestMessageText {
        id: String,
        message_index: u64,
        media_index: Option<u64>,
    },
    /// `agents/completions/request/messages/image/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_image`].
    AgentCompletionRequestMessageImage {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/messages/audio/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_audio`].
    AgentCompletionRequestMessageAudio {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/messages/video/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_video`].
    AgentCompletionRequestMessageVideo {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/messages/file/<id>_<msg>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_file`].
    AgentCompletionRequestMessageFile {
        id: String,
        message_index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/messages/assistant/reasoning/<id>_<msg>.json`
    /// (a JSON-encoded string — unlike the response side's raw `.txt`)
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_assistant_reasoning`].
    AgentCompletionRequestMessageAssistantReasoning {
        id: String,
        message_index: u64,
    },
    /// `agents/completions/request/messages/assistant/refusal/<id>_<msg>.json`
    /// (a JSON-encoded string — unlike the response side's raw `.txt`)
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_assistant_refusal`].
    AgentCompletionRequestMessageAssistantRefusal {
        id: String,
        message_index: u64,
    },
    /// `agents/completions/request/messages/assistant/tool_calls/<id>_<msg>_<tc>.json`
    /// (a full `AssistantToolCall`, not a delta)
    /// — [`crate::filesystem::Client::read_agent_completion_request_message_assistant_tool_call`].
    AgentCompletionRequestMessageAssistantToolCall {
        id: String,
        message_index: u64,
        tool_call_index: u64,
    },

    // -- Notification content ----------------------------------------------
    /// `agents/completions/request/notifications/text/<response_id>_<idx>[_<part>].txt`
    /// — [`crate::filesystem::Client::read_agent_completion_notification_text`].
    AgentCompletionNotificationText {
        response_id: String,
        index: u64,
        media_index: Option<u64>,
    },
    /// `agents/completions/request/notifications/image/<response_id>_<idx>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_notification_image`].
    AgentCompletionNotificationImage {
        response_id: String,
        index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/notifications/audio/<response_id>_<idx>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_notification_audio`].
    AgentCompletionNotificationAudio {
        response_id: String,
        index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/notifications/video/<response_id>_<idx>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_notification_video`].
    AgentCompletionNotificationVideo {
        response_id: String,
        index: u64,
        media_index: u64,
    },
    /// `agents/completions/request/notifications/file/<response_id>_<idx>_<part>.<ext>`
    /// — [`crate::filesystem::Client::read_agent_completion_notification_file`].
    AgentCompletionNotificationFile {
        response_id: String,
        index: u64,
        media_index: u64,
    },
}

impl LogFileKind {
    /// Classify a logs-relative path into a typed read descriptor.
    /// Returns `None` for paths that don't match any known writer
    /// pattern.
    ///
    /// The match is on the directory prefix; the stem is then split
    /// into its underscore-separated trailing-integer suffix(es) to
    /// recover `message_index` / `media_index` / `tool_call_index`.
    /// The leading portion of the stem — `id` or `response_id` — is
    /// taken verbatim (it may itself contain underscores).
    pub fn from_path(rel_path: &str) -> Option<Self> {
        let (dir, stem) = split_path(rel_path)?;

        match dir {
            // -- Top-level envelopes (stem == id) -----------------------
            "agents/completions/response" => Some(Self::AgentCompletion {
                id: stem.to_string(),
            }),
            "agents/completions/request" => {
                Some(Self::AgentCompletionRequest {
                    id: stem.to_string(),
                })
            }
            "agents/completions/response/continuation" => {
                Some(Self::AgentCompletionContinuation {
                    id: stem.to_string(),
                })
            }
            "agents/completions/request/continuation" => {
                Some(Self::AgentCompletionRequestContinuation {
                    id: stem.to_string(),
                })
            }
            "vector/completions/response" => Some(Self::VectorCompletion {
                id: stem.to_string(),
            }),
            "vector/completions/request" => {
                Some(Self::VectorCompletionRequest {
                    id: stem.to_string(),
                })
            }
            "functions/executions/response" => Some(Self::FunctionExecution {
                id: stem.to_string(),
            }),
            "functions/executions/request" => {
                Some(Self::FunctionExecutionRequest {
                    id: stem.to_string(),
                })
            }
            "functions/executions/response/retry_token" => {
                Some(Self::FunctionExecutionRetryToken {
                    id: stem.to_string(),
                })
            }
            "functions/inventions/response" => Some(Self::FunctionInvention {
                id: stem.to_string(),
            }),
            "functions/inventions/request" => {
                Some(Self::FunctionInventionRequest {
                    id: stem.to_string(),
                })
            }
            "functions/inventions/recursive/response" => {
                Some(Self::FunctionInventionRecursive {
                    id: stem.to_string(),
                })
            }
            "functions/inventions/recursive/request" => {
                Some(Self::FunctionInventionRecursiveRequest {
                    id: stem.to_string(),
                })
            }

            // -- Assistant content (response side, under .../messages/assistant/) -
            // The response-side writer at
            // `crate::logs::agents::completions::response::streaming::assistant_response_chunk`
            // routes assistant-role chunks under a kind-specific
            // `messages/assistant/<sub>` subdir so the (response_id,
            // index) stems can't collide with a tool message at the
            // same index. Same stem shapes as the bare-prefix
            // branches above; the prefix carries the role.
            "agents/completions/response/messages/assistant" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionMessageAssistant {
                    id,
                    message_index,
                })
            }
            "agents/completions/response/messages/assistant/logprobs" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionMessageAssistantLogprobs { id, message_index })
            }
            "agents/completions/response/messages/assistant/reasoning" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionMessageAssistantReasoning {
                    id,
                    message_index,
                })
            }
            "agents/completions/response/messages/assistant/refusal" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionMessageAssistantRefusal { id, message_index })
            }
            // stem == "id_msg_tc" — peel TWO trailing u64s.
            "agents/completions/response/messages/assistant/tool_calls" => {
                let (rest, tool_call_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageAssistantToolCall {
                    id,
                    message_index,
                    tool_call_index,
                })
            }
            "agents/completions/response/messages/assistant/text" => {
                let (id, message_index, media_index) = peel_text_stem(stem)?;
                Some(Self::AgentCompletionMessageAssistantText {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/assistant/image" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageAssistantImage {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/assistant/audio" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageAssistantAudio {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/assistant/video" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageAssistantVideo {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/assistant/file" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageAssistantFile {
                    id,
                    message_index,
                    media_index,
                })
            }

            // -- Tool response content (under .../messages/tool/) ---------
            // The ToolResponse envelope — written by
            // `crate::logs::agents::completions::response::tool_response`
            // at `messages/tool/<id>_<msg>.json`, mirrored by
            // `crate::filesystem::db::schema::message_kind_file_path`.
            "agents/completions/response/messages/tool" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionMessageTool { id, message_index })
            }
            "agents/completions/response/messages/tool/text" => {
                let (id, message_index, media_index) = peel_text_stem(stem)?;
                Some(Self::AgentCompletionMessageToolText {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/tool/image" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageToolImage {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/tool/audio" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageToolAudio {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/tool/video" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageToolVideo {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/response/messages/tool/file" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionMessageToolFile {
                    id,
                    message_index,
                    media_index,
                })
            }

            // -- Request-side message content ----------------------------
            // One bare `messages/` namespace for every role — no
            // role subdirs on the request side (see the variant doc).
            "agents/completions/request/messages" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionRequestMessage {
                    id,
                    message_index,
                })
            }
            "agents/completions/request/messages/text" => {
                let (id, message_index, media_index) = peel_text_stem(stem)?;
                Some(Self::AgentCompletionRequestMessageText {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/request/messages/image" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionRequestMessageImage {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/request/messages/audio" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionRequestMessageAudio {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/request/messages/video" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionRequestMessageVideo {
                    id,
                    message_index,
                    media_index,
                })
            }
            "agents/completions/request/messages/file" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionRequestMessageFile {
                    id,
                    message_index,
                    media_index,
                })
            }
            // Request-side assistant extras — written by
            // `crate::logs::agents::completions::message::assistant_message`
            // under the role-specific `messages/assistant/` subdir, as
            // JSON-encoded strings (unlike the response side's raw
            // `.txt`). Their `LogReference`s are file-id-minted by
            // `Client::message_log_to_queue_message`, so they must be
            // classifiable for `read_file_by_id`.
            "agents/completions/request/messages/assistant/reasoning" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionRequestMessageAssistantReasoning {
                    id,
                    message_index,
                })
            }
            "agents/completions/request/messages/assistant/refusal" => {
                let (id, message_index) = peel_u64(stem)?;
                Some(Self::AgentCompletionRequestMessageAssistantRefusal {
                    id,
                    message_index,
                })
            }
            // stem == "id_msg_tc" — peel TWO trailing u64s.
            "agents/completions/request/messages/assistant/tool_calls" => {
                let (rest, tool_call_index) = peel_u64(stem)?;
                let (id, message_index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionRequestMessageAssistantToolCall {
                    id,
                    message_index,
                    tool_call_index,
                })
            }

            // -- Notification content ------------------------------------
            "agents/completions/request/notifications/text" => {
                let (response_id, index, media_index) = peel_text_stem(stem)?;
                Some(Self::AgentCompletionNotificationText {
                    response_id,
                    index,
                    media_index,
                })
            }
            "agents/completions/request/notifications/image" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (response_id, index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionNotificationImage {
                    response_id,
                    index,
                    media_index,
                })
            }
            "agents/completions/request/notifications/audio" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (response_id, index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionNotificationAudio {
                    response_id,
                    index,
                    media_index,
                })
            }
            "agents/completions/request/notifications/video" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (response_id, index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionNotificationVideo {
                    response_id,
                    index,
                    media_index,
                })
            }
            "agents/completions/request/notifications/file" => {
                let (rest, media_index) = peel_u64(stem)?;
                let (response_id, index) = peel_u64(&rest)?;
                Some(Self::AgentCompletionNotificationFile {
                    response_id,
                    index,
                    media_index,
                })
            }

            _ => None,
        }
    }
}

/// Split a logs-relative path into `(dir, stem)`. Drops the extension.
/// `agents/completions/.../foo.json` → `("agents/completions/...", "foo")`.
fn split_path(rel_path: &str) -> Option<(&str, &str)> {
    let (head, filename) = rel_path.rsplit_once('/')?;
    let stem = filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(filename);
    Some((head, stem))
}

/// Peel one trailing `_<u64>` segment off `stem`. Returns `(head, n)`.
/// `"abc_def_42"` → `("abc_def", 42)`. `None` if the trailing segment
/// doesn't parse as `u64` or if there's no underscore.
fn peel_u64(stem: &str) -> Option<(String, u64)> {
    let (head, tail) = stem.rsplit_once('_')?;
    let n = u64::from_str(tail).ok()?;
    Some((head.to_string(), n))
}

/// Stem layout for the text-content variants: either `<id>_<msg>`
/// (single `RichContent::Text(_)`, no `media_index`) or
/// `<id>_<msg>_<part>` (`RichContent::Parts([..., Text { text }, ...])`
/// — one file per part).
///
/// Strategy: peel one trailing u64. If a second u64 also peels off,
/// the inner one is `message_index` and the outer is `media_index`.
/// If not, the single peeled one is `message_index` and there is no
/// `media_index`.
fn peel_text_stem(stem: &str) -> Option<(String, u64, Option<u64>)> {
    let (head, last) = peel_u64(stem)?;
    if let Some((id, message_index)) = peel_u64(&head) {
        Some((id, message_index, Some(last)))
    } else {
        Some((head, last, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_completion_envelope() {
        let k =
            LogFileKind::from_path("agents/completions/response/acc-1.json")
                .unwrap();
        assert_eq!(k, LogFileKind::AgentCompletion { id: "acc-1".into() });
    }

    #[test]
    fn agent_completion_request_envelope() {
        let k = LogFileKind::from_path("agents/completions/request/acc-1.json")
            .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequest { id: "acc-1".into() }
        );
    }

    #[test]
    fn assistant_message_envelope() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/acc-1_0.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistant {
                id: "acc-1".into(),
                message_index: 0
            }
        );
    }

    #[test]
    fn tool_message_envelope() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/tool/acc-1_1.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageTool {
                id: "acc-1".into(),
                message_index: 1
            }
        );
    }

    #[test]
    fn agent_completion_message_reasoning() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/reasoning/acc-1_0.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantReasoning {
                id: "acc-1".into(),
                message_index: 0
            }
        );
    }

    #[test]
    fn agent_completion_message_tool_call() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/tool_calls/acc-1_0_2.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantToolCall {
                id: "acc-1".into(),
                message_index: 0,
                tool_call_index: 2
            }
        );
    }

    #[test]
    fn agent_completion_message_image() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/image/acc-1_0_3.png",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantImage {
                id: "acc-1".into(),
                message_index: 0,
                media_index: 3
            }
        );
    }

    #[test]
    fn assistant_text_single_part() {
        // RichContent::Text → no media_index.
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/text/acc-1_0.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantText {
                id: "acc-1".into(),
                message_index: 0,
                media_index: None
            }
        );
    }

    #[test]
    fn assistant_text_parts() {
        // RichContent::Parts text part → media_index = part_idx.
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/text/acc-1_0_2.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantText {
                id: "acc-1".into(),
                message_index: 0,
                media_index: Some(2)
            }
        );
    }

    #[test]
    fn legacy_bare_response_paths_unclassified() {
        // The pre-role-subdir writer routes are gone; their classifier
        // branches were removed with them.
        for path in [
            "agents/completions/response/messages/acc-1_0.json",
            "agents/completions/response/messages/logprobs/acc-1_0.json",
            "agents/completions/response/messages/reasoning/acc-1_0.txt",
            "agents/completions/response/messages/refusal/acc-1_0.txt",
            "agents/completions/response/messages/tool_calls/acc-1_0_2.json",
            "agents/completions/response/messages/text/acc-1_0.txt",
            "agents/completions/response/messages/image/acc-1_0_3.png",
            "agents/completions/response/messages/audio/acc-1_0_3.wav",
            "agents/completions/response/messages/video/acc-1_0_3.mp4",
            "agents/completions/response/messages/file/acc-1_0_3.pdf",
        ] {
            assert_eq!(LogFileKind::from_path(path), None, "{path}");
        }
    }

    #[test]
    fn request_message_assistant_reasoning() {
        let k = LogFileKind::from_path(
            "agents/completions/request/messages/assistant/reasoning/acc-1_2.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestMessageAssistantReasoning {
                id: "acc-1".into(),
                message_index: 2
            }
        );
    }

    #[test]
    fn request_message_assistant_refusal() {
        let k = LogFileKind::from_path(
            "agents/completions/request/messages/assistant/refusal/acc-1_2.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestMessageAssistantRefusal {
                id: "acc-1".into(),
                message_index: 2
            }
        );
    }

    #[test]
    fn request_message_envelope() {
        // All roles share the bare request `messages/` namespace.
        let k = LogFileKind::from_path(
            "agents/completions/request/messages/acc-1_3.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestMessage {
                id: "acc-1".into(),
                message_index: 3
            }
        );
    }

    #[test]
    fn request_message_assistant_tool_call() {
        let k = LogFileKind::from_path(
            "agents/completions/request/messages/assistant/tool_calls/acc-1_0_2.json",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestMessageAssistantToolCall {
                id: "acc-1".into(),
                message_index: 0,
                tool_call_index: 2
            }
        );
    }

    #[test]
    fn request_continuation() {
        let k = LogFileKind::from_path(
            "agents/completions/request/continuation/acc-1.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestContinuation {
                id: "acc-1".into()
            }
        );
    }

    #[test]
    fn tool_response_text_single() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/tool/text/acc-1_0.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageToolText {
                id: "acc-1".into(),
                message_index: 0,
                media_index: None
            }
        );
    }

    #[test]
    fn tool_response_image() {
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/tool/image/acc-1_0_2.png",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageToolImage {
                id: "acc-1".into(),
                message_index: 0,
                media_index: 2
            }
        );
    }

    #[test]
    fn request_text_single() {
        let k = LogFileKind::from_path(
            "agents/completions/request/messages/text/acc-1_0.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionRequestMessageText {
                id: "acc-1".into(),
                message_index: 0,
                media_index: None
            }
        );
    }

    #[test]
    fn notification_text_parts() {
        let k = LogFileKind::from_path(
            "agents/completions/request/notifications/text/acc-1_5_2.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionNotificationText {
                response_id: "acc-1".into(),
                index: 5,
                media_index: Some(2)
            }
        );
    }

    #[test]
    fn function_execution_request() {
        let k =
            LogFileKind::from_path("functions/executions/request/fer-9.json")
                .unwrap();
        assert_eq!(
            k,
            LogFileKind::FunctionExecutionRequest { id: "fer-9".into() }
        );
    }

    #[test]
    fn id_with_internal_underscore() {
        // The id may itself contain underscores — only the trailing
        // integer-shaped segments are indices.
        let k = LogFileKind::from_path(
            "agents/completions/response/messages/assistant/reasoning/my_id_with_dashes_7.txt",
        )
        .unwrap();
        assert_eq!(
            k,
            LogFileKind::AgentCompletionMessageAssistantReasoning {
                id: "my_id_with_dashes".into(),
                message_index: 7
            }
        );
    }

    #[test]
    fn unknown_path_returns_none() {
        assert_eq!(LogFileKind::from_path("foo/bar/baz.txt"), None);
        assert_eq!(LogFileKind::from_path(""), None);
        assert_eq!(LogFileKind::from_path("noslash.json"), None);
    }
}
