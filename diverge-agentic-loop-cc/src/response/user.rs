//! The `user` records: what went back to the model.

use diverge_provider_sdk::shared::containers::run_loop::response;
use serde::Deserialize;

use super::message;

/// A `type: "user"` record: a user turn or a tool result, echoed to
/// stdout — one content block per record, like the assistant's.
///
/// # One struct for the schema's two
///
/// The source defines `SDKUserMessage` and `SDKUserMessageReplay` as
/// separate schemas differing only in obligation: a replay requires
/// `uuid`, `session_id` and `isReplay: true`, the plain one makes the
/// ids optional and has no flag. One struct with those fields
/// optional covers both arms; [`is_replay`](Self::is_replay) present
/// and true is what a replay is.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct User {
    /// Always `user`.
    pub r#type: UserType,
    /// The message params: role and content, the request vocabulary.
    pub message: message::UserMessage,
    /// The spawning tool call, for a subagent's record; `null` on
    /// the main thread.
    pub parent_tool_use_id: Option<String>,
    /// Set on messages Claude Code made up itself — meta turns and
    /// transcript-only annotations.
    #[serde(
        rename = "isSynthetic"
    )]
    pub is_synthetic: Option<bool>,
    /// The tool's result in Claude Code's own richer form, beside
    /// the API-shaped block. `unknown` in the schema, and kept so.
    pub tool_use_result: Option<serde_json::Value>,
    /// Queueing priority, when the message was queued.
    pub priority: Option<Priority>,
    /// When the message was created on the originating process, ISO
    /// 8601; older emitters omit it.
    pub timestamp: Option<String>,
    /// Marks an echo of something already said — a resumed history
    /// replay or an acknowledgement — rather than a new turn.
    #[serde(
        rename = "isReplay"
    )]
    pub is_replay: Option<bool>,
    /// The record's own id; optional on the plain arm.
    pub uuid: Option<String>,
    /// The session; optional on the plain arm.
    pub session_id: Option<String>,
}

/// A queued message's priority.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Interrupt for it.
    Now,
    /// After the current step.
    Next,
    /// Whenever.
    Later,
}

/// The `user` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    /// The only value.
    #[default]
    User,
}

impl User {
    /// This record's chunks: its tool results.
    ///
    /// Own fields first. A replay — [`is_replay`](Self::is_replay)
    /// true — is the reader's: its tap has already resolved the
    /// fate and pushed the `user` chunk before conversion runs, so
    /// here it converts to nothing. Everything else delegates to
    /// the message, attributed: a record with a non-null
    /// [`parent_tool_use_id`](Self::parent_tool_use_id) is a
    /// sub-agent's, and its tool results carry that id as their
    /// `parent_tool_call_id`. And when the message yields nothing
    /// but the record carries a subagent's OUTCOME —
    /// [`tool_use_result`](Self::tool_use_result) beside the parent
    /// id — the api crate's fallback applies: the outcome's JSON
    /// answers the spawning call, as one text block. That response
    /// is the MAIN thread's, deliberately: its `id` IS the spawning
    /// call, and it answers on the thread that made the call.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        if self.is_replay == Some(true) {
            return;
        }
        let start = chunks.len();
        self.message
            .into_chunks(chunks, self.parent_tool_use_id.as_deref());
        if chunks.len() == start {
            if let (Some(result), Some(id)) =
                (self.tool_use_result, self.parent_tool_use_id)
            {
                chunks.push(response::AgenticLoopChunk::ToolResponse(
                    response::ToolResponseChunk {
                        r#type: Default::default(),
                        parent_tool_call_id: None,
                        id,
                        inner: rmcp::model::CallToolResult::success(vec![
                            rmcp::model::ContentBlock::text(
                                result.to_string(),
                            ),
                        ]),
                    },
                ));
            }
        }
    }
}
