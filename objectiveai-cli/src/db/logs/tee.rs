//! The log writer's live-conversation tee.
//!
//! When [`super::writer`] is about to write a row (its shadow said
//! `Insert`/`Update`), it also hands an OWNED copy of that row to the
//! [`ConversationTee`] — a keyed FULL-VALUE frame, exactly what the DB
//! is being told, shipped to the resident daemon's fixed-name
//! `conversation.sock` for fan-out to `/agents/instances/{*aih}`
//! subscribers. The tee is strictly best-effort and NEVER blocks the
//! writer: `send` is an unbounded-mpsc push, and all socket I/O
//! (connect, retries, backpressure, daemon-absent) lives on a
//! detached RX task that drops frames when the daemon is unreachable.
//!
//! Frames are teed BEFORE the row's SQL executes, so live delivery is
//! not gated on DB latency; a row whose INSERT later fails was still
//! shipped (accepted divergence — reconnecting clients replay DB
//! truth). One tee (one socket connection) carries rows for MANY
//! AIHs — a function execution's writer streams every nested agent's
//! rows — so each frame is tagged with its own row's AIH and the
//! daemon routes per-frame, never per-connection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::Name;
use interprocess::local_socket::tokio::prelude::*;
use objectiveai_sdk::cli::websocket_agents_instances_listener::{
    AgentInstanceEvent, AssistantResponsePart, PartContent, RequestMessageUserPart,
    ToolResponsePart, VectorRequestChoicePart,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::row::RowValue;

/// One JSONL line on `conversation.sock`. CLI-internal (the daemon is
/// the only reader); the resolved wire type the daemon fans out is the
/// SDK's `AgentInstanceEvent`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TeeFrame {
    /// A fully-resolved conversation event, fanned out verbatim.
    Event { event: AgentInstanceEvent },
    /// A consumed message-queue notification. The writer only knows
    /// the `message_queue_contents.id` — the content and the parent
    /// queue row's metadata live in the DB, so the DAEMON resolves
    /// them into the typed `ClientNotificationPart` event before
    /// fan-out (notifications are low-frequency).
    MessageQueueContent {
        agent_instance_hierarchy: String,
        response_id: String,
        message_queue_content_id: i64,
        /// RFC3339 — when the writer shipped this frame.
        delivered_at: String,
    },
}

/// Build the tee frame for one logged error — persist-before-return:
/// the spawn path calls this AFTER `insert_error` committed the row.
pub fn error_frame(
    agent_instance_hierarchy: String,
    response_id: Option<String>,
    error: serde_json::Value,
    created_at: i64,
) -> TeeFrame {
    TeeFrame::Event {
        event: AgentInstanceEvent::Error {
            agent_instance_hierarchy,
            response_id,
            error,
            delivered_at: crate::db::time::unix_to_rfc3339(created_at),
        },
    }
}

/// The writer's row→event mapper. STATEFUL: head rows (the
/// `tool_response` / `request_message_tool` / `request_vector_choice`
/// metadata carriers) produce NO frame — they feed the maps below, and
/// their payload rides every subsequent content event of their block
/// (`tool_call_id` on the event's boundary fields; the choice's voting
/// `key` per part). The writer emits each head strictly before its
/// contents, so a lookup miss means a torn iterator — that content
/// frame is skipped (reconnecting clients replay DB truth).
#[derive(Default)]
pub struct FrameMapper {
    /// `(aih, response_id, message index)` → `tool_call_id`, from
    /// `tool_response` heads. The AIH is part of the key because ONE
    /// writer streams MANY agents' rows (a function execution's
    /// writer carries every nested agent).
    tool_response_heads: HashMap<(String, String, i64), String>,
    /// Same, from `request_message_tool` heads.
    request_tool_heads: HashMap<(String, String, i64), String>,
    /// `(aih, response_id, choice index)` → the agent's voting key,
    /// from `request_vector_choice` heads.
    choice_keys: HashMap<(String, String, i64), String>,
}

impl FrameMapper {
    /// Map one row the writer is about to write into its typed event
    /// frame. `created_at` is the same timestamp the SQL will bind.
    /// `None` for head rows (memory only) and for content rows whose
    /// head never registered.
    pub fn map(&mut self, value: &RowValue<'_>, created_at: i64) -> Option<TeeFrame> {
        let delivered_at = crate::db::time::unix_to_rfc3339(created_at);
        let aih = value.agent_instance_hierarchy().to_string();
        let event = match value {
            // ---- daemon-resolved notification rows ----
            RowValue::MessageQueueContent {
                response_id,
                agent_instance_hierarchy,
                message_queue_content_id,
            } => {
                return Some(TeeFrame::MessageQueueContent {
                    agent_instance_hierarchy: (*agent_instance_hierarchy).to_string(),
                    response_id: (*response_id).to_string(),
                    message_queue_content_id: *message_queue_content_id,
                    delivered_at,
                });
            }

            // ---- head rows: memory only, no frame ----
            RowValue::ToolResponse { tool_call_id, .. } => {
                self.tool_response_heads.insert(
                    (aih, value.response_id().to_string(), value.row_index()),
                    (*tool_call_id).to_string(),
                );
                return None;
            }
            RowValue::RequestMessageTool { tool_call_id, .. } => {
                self.request_tool_heads.insert(
                    (aih, value.response_id().to_string(), value.row_index()),
                    (*tool_call_id).to_string(),
                );
                return None;
            }
            RowValue::RequestVectorChoice { key, .. } => {
                self.choice_keys.insert(
                    (aih, value.response_id().to_string(), value.row_index()),
                    (*key).to_string(),
                );
                return None;
            }

            // ---- assistant response ----
            RowValue::AssistantResponseRefusal { text, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Refusal { delivered_at, text: (*text).to_string() },
            ),
            RowValue::AssistantResponseReasoning { text, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Reasoning { delivered_at, text: (*text).to_string() },
            ),
            RowValue::AssistantResponseToolCalls {
                tool_call_id, function_name, arguments, ..
            } => assistant_event(
                value, false,
                AssistantResponsePart::ToolCall {
                    delivered_at,
                    function_name: (*function_name).to_string(),
                    tool_call_id: (*tool_call_id).to_string(),
                    tool_call_index: value.row_sub_index().unwrap_or(0),
                    arguments: (*arguments).to_string(),
                },
            ),
            RowValue::AssistantResponseContentText { text, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Text { delivered_at, text: (*text).to_string() },
            ),
            RowValue::AssistantResponseContentImage { image_url, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Image { delivered_at, image: (*image_url).clone() },
            ),
            RowValue::AssistantResponseContentAudio { input_audio, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Audio { delivered_at, audio: (*input_audio).clone() },
            ),
            RowValue::AssistantResponseContentVideo { video_url, .. } => assistant_event(
                value, false,
                AssistantResponsePart::Video { delivered_at, video: (*video_url).clone() },
            ),
            RowValue::AssistantResponseContentFile { file, .. } => assistant_event(
                value, false,
                AssistantResponsePart::File { delivered_at, file: (*file).clone() },
            ),

            // ---- request message: assistant (same part shape) ----
            RowValue::RequestMessageAssistantRefusal { text, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Refusal { delivered_at, text: (*text).to_string() },
            ),
            RowValue::RequestMessageAssistantReasoning { text, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Reasoning { delivered_at, text: (*text).to_string() },
            ),
            RowValue::RequestMessageAssistantToolCalls {
                tool_call_id, function_name, arguments, ..
            } => assistant_event(
                value, true,
                AssistantResponsePart::ToolCall {
                    delivered_at,
                    function_name: (*function_name).to_string(),
                    tool_call_id: (*tool_call_id).to_string(),
                    tool_call_index: value.row_sub_index().unwrap_or(0),
                    arguments: (*arguments).to_string(),
                },
            ),
            RowValue::RequestMessageAssistantContentText { text, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Text { delivered_at, text: (*text).to_string() },
            ),
            RowValue::RequestMessageAssistantContentImage { image_url, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Image { delivered_at, image: (*image_url).clone() },
            ),
            RowValue::RequestMessageAssistantContentAudio { input_audio, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Audio { delivered_at, audio: (*input_audio).clone() },
            ),
            RowValue::RequestMessageAssistantContentVideo { video_url, .. } => assistant_event(
                value, true,
                AssistantResponsePart::Video { delivered_at, video: (*video_url).clone() },
            ),
            RowValue::RequestMessageAssistantContentFile { file, .. } => assistant_event(
                value, true,
                AssistantResponsePart::File { delivered_at, file: (*file).clone() },
            ),

            // ---- tool response content (needs its head's id) ----
            RowValue::ToolResponseContentText { text, .. } => self.tool_event(
                value, delivered_at, PartContent::Text { text: (*text).to_string() }, false,
            )?,
            RowValue::ToolResponseContentImage { image_url, .. } => self.tool_event(
                value, delivered_at, PartContent::Image((*image_url).clone()), false,
            )?,
            RowValue::ToolResponseContentAudio { input_audio, .. } => self.tool_event(
                value, delivered_at, PartContent::Audio((*input_audio).clone()), false,
            )?,
            RowValue::ToolResponseContentVideo { video_url, .. } => self.tool_event(
                value, delivered_at, PartContent::Video((*video_url).clone()), false,
            )?,
            RowValue::ToolResponseContentFile { file, .. } => self.tool_event(
                value, delivered_at, PartContent::File((*file).clone()), false,
            )?,
            RowValue::RequestMessageToolContentText { text, .. } => self.tool_event(
                value, delivered_at, PartContent::Text { text: (*text).to_string() }, true,
            )?,
            RowValue::RequestMessageToolContentImage { image_url, .. } => self.tool_event(
                value, delivered_at, PartContent::Image((*image_url).clone()), true,
            )?,
            RowValue::RequestMessageToolContentAudio { input_audio, .. } => self.tool_event(
                value, delivered_at, PartContent::Audio((*input_audio).clone()), true,
            )?,
            RowValue::RequestMessageToolContentVideo { video_url, .. } => self.tool_event(
                value, delivered_at, PartContent::Video((*video_url).clone()), true,
            )?,
            RowValue::RequestMessageToolContentFile { file, .. } => self.tool_event(
                value, delivered_at, PartContent::File((*file).clone()), true,
            )?,

            // ---- request message: user ----
            RowValue::RequestMessageUserContentText { text, .. } => user_event(
                value,
                RequestMessageUserPart {
                    delivered_at,
                    content: PartContent::Text { text: (*text).to_string() },
                },
            ),
            RowValue::RequestMessageUserContentImage { image_url, .. } => user_event(
                value,
                RequestMessageUserPart {
                    delivered_at,
                    content: PartContent::Image((*image_url).clone()),
                },
            ),
            RowValue::RequestMessageUserContentAudio { input_audio, .. } => user_event(
                value,
                RequestMessageUserPart {
                    delivered_at,
                    content: PartContent::Audio((*input_audio).clone()),
                },
            ),
            RowValue::RequestMessageUserContentVideo { video_url, .. } => user_event(
                value,
                RequestMessageUserPart {
                    delivered_at,
                    content: PartContent::Video((*video_url).clone()),
                },
            ),
            RowValue::RequestMessageUserContentFile { file, .. } => user_event(
                value,
                RequestMessageUserPart {
                    delivered_at,
                    content: PartContent::File((*file).clone()),
                },
            ),

            // ---- vector request choice content (needs its head's key) ----
            RowValue::RequestVectorChoiceContentText { text, .. } => self.choice_event(
                value, delivered_at, PartContent::Text { text: (*text).to_string() },
            )?,
            RowValue::RequestVectorChoiceContentImage { image_url, .. } => self.choice_event(
                value, delivered_at, PartContent::Image((*image_url).clone()),
            )?,
            RowValue::RequestVectorChoiceContentAudio { input_audio, .. } => self.choice_event(
                value, delivered_at, PartContent::Audio((*input_audio).clone()),
            )?,
            RowValue::RequestVectorChoiceContentVideo { video_url, .. } => self.choice_event(
                value, delivered_at, PartContent::Video((*video_url).clone()),
            )?,
            RowValue::RequestVectorChoiceContentFile { file, .. } => self.choice_event(
                value, delivered_at, PartContent::File((*file).clone()),
            )?,

            // ---- vector response vote (complete single-row block) ----
            RowValue::ResponseVectorVote { vote, .. } => {
                AgentInstanceEvent::VectorResponseVote {
                    agent_instance_hierarchy: aih,
                    response_id: value.response_id().to_string(),
                    vote: vote.to_vec(),
                }
            }
        };
        Some(TeeFrame::Event { event })
    }

    /// A tool-response / request-tool content event — `tool_call_id`
    /// from the block's registered head.
    fn tool_event(
        &self,
        value: &RowValue<'_>,
        delivered_at: String,
        content: PartContent,
        request: bool,
    ) -> Option<AgentInstanceEvent> {
        let response_id = value.response_id().to_string();
        let row_index = value.row_index();
        let heads = if request {
            &self.request_tool_heads
        } else {
            &self.tool_response_heads
        };
        let aih = value.agent_instance_hierarchy().to_string();
        let tool_call_id = heads.get(&(aih, response_id.clone(), row_index))?.clone();
        let part = ToolResponsePart {
            delivered_at,
            content,
        };
        Some(if request {
            AgentInstanceEvent::RequestMessageToolPart {
                agent_instance_hierarchy: value.agent_instance_hierarchy().to_string(),
                response_id,
                tool_call_id,
                row_index,
                row_sub_index: value.row_sub_index(),
                part,
            }
        } else {
            AgentInstanceEvent::ToolResponsePart {
                agent_instance_hierarchy: value.agent_instance_hierarchy().to_string(),
                response_id,
                tool_call_id,
                row_index,
                row_sub_index: value.row_sub_index(),
                part,
            }
        })
    }

    /// A vector-choice content event — the voting `key` from the
    /// choice's registered head.
    fn choice_event(
        &self,
        value: &RowValue<'_>,
        delivered_at: String,
        content: PartContent,
    ) -> Option<AgentInstanceEvent> {
        let response_id = value.response_id().to_string();
        let choice_index = value.row_index();
        let key = self
            .choice_keys
            .get(&(
                value.agent_instance_hierarchy().to_string(),
                response_id.clone(),
                choice_index,
            ))?
            .clone();
        Some(AgentInstanceEvent::VectorRequestChoicePart {
            agent_instance_hierarchy: value.agent_instance_hierarchy().to_string(),
            response_id,
            key,
            choice_index,
            part_index: value.row_sub_index().unwrap_or(0),
            part: VectorRequestChoicePart {
                delivered_at,
                content,
            },
        })
    }
}

/// An assistant-part event, response (`request == false`) or request
/// (`request == true`) side.
fn assistant_event(
    value: &RowValue<'_>,
    request: bool,
    part: AssistantResponsePart,
) -> AgentInstanceEvent {
    let agent_instance_hierarchy = value.agent_instance_hierarchy().to_string();
    let response_id = value.response_id().to_string();
    let row_index = value.row_index();
    let row_sub_index = value.row_sub_index();
    if request {
        AgentInstanceEvent::RequestMessageAssistantPart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part,
        }
    } else {
        AgentInstanceEvent::AssistantResponsePart {
            agent_instance_hierarchy,
            response_id,
            row_index,
            row_sub_index,
            part,
        }
    }
}

/// A user-part event.
fn user_event(value: &RowValue<'_>, part: RequestMessageUserPart) -> AgentInstanceEvent {
    AgentInstanceEvent::RequestMessageUserPart {
        agent_instance_hierarchy: value.agent_instance_hierarchy().to_string(),
        response_id: value.response_id().to_string(),
        row_index: value.row_index(),
        row_sub_index: value.row_sub_index(),
        part,
    }
}

/// The fixed local-socket name for the daemon's conversation hub —
/// MUST match the listener side in
/// `crate::websockets::websocket_agent_instance::socket_name`.
/// Mirrors the `daemon.sock` / `agents.sock` scheme with the constant
/// `conversation`.
#[cfg(unix)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    crate::websockets::mcp_listener::socks_dir(state_dir)
        .join("conversation.sock")
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(state_dir: &Path) -> std::io::Result<Name<'static>> {
    use std::hash::{Hash, Hasher};
    // Named pipes are machine-global; fold the state NAME into the
    // pipe name to preserve per-state isolation (matching
    // `daemon_stream` / `websocket_agents` / `mcp_listener`).
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state_dir.file_name().hash(&mut hasher);
    let state = hasher.finish();
    format!("objectiveai-{state:016x}-conversation.sock").to_ns_name::<GenericNamespaced>()
}

/// Handle held by the log writer. Cloneable — one spawn's restart
/// passes share one tee (one socket connection). Dropping every clone
/// closes the channel; the RX task drains and exits.
#[derive(Clone)]
pub struct ConversationTee {
    tx: mpsc::UnboundedSender<TeeFrame>,
}

impl ConversationTee {
    /// Create the tee and detach its RX task. The task owns the
    /// receiver and the socket connection; all I/O failure modes
    /// (daemon absent, mid-stream write error) degrade to dropped
    /// frames, never to writer backpressure.
    pub fn spawn(state_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(rx_task(state_dir, rx));
        Self { tx }
    }

    /// Fire-and-forget. A dead RX task (should not happen while any
    /// clone lives) means the frame is silently dropped.
    pub fn send(&self, frame: TeeFrame) {
        let _ = self.tx.send(frame);
    }
}

/// Drain the channel into `conversation.sock` as JSONL. Lazy connect
/// on the first frame; on failure (daemon absent) frames are dropped
/// and reconnect attempts are gated to once per second — cheap
/// recovery after a daemon restart without a per-row connect storm.
async fn rx_task(state_dir: PathBuf, mut rx: mpsc::UnboundedReceiver<TeeFrame>) {
    let mut write: Option<tokio::io::WriteHalf<LocalSocketStream>> = None;
    let mut last_attempt: Option<Instant> = None;
    while let Some(frame) = rx.recv().await {
        let Ok(mut line) = serde_json::to_vec(&frame) else {
            continue;
        };
        line.push(b'\n');
        if write.is_none() {
            let due = last_attempt
                .map(|at| at.elapsed() >= Duration::from_secs(1))
                .unwrap_or(true);
            if due {
                last_attempt = Some(Instant::now());
                write = connect(&state_dir).await;
            }
        }
        let Some(sink) = write.as_mut() else {
            // No daemon reachable — drop the frame (best-effort).
            continue;
        };
        if sink.write_all(&line).await.is_err() || sink.flush().await.is_err() {
            // Connection died (daemon restart) — drop this frame;
            // later frames retry through the 1s gate above.
            write = None;
        }
    }
}

/// Connect to the daemon's conversation socket. The ONE retried error
/// is Windows `ERROR_PIPE_BUSY` (a live listener mid-accept), same
/// rationale as `daemon_stream::connect_feed`.
async fn connect(state_dir: &Path) -> Option<tokio::io::WriteHalf<LocalSocketStream>> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let mut attempts = 0u32;
    loop {
        let name = socket_name(state_dir).ok()?;
        match LocalSocketStream::connect(name).await {
            Ok(conn) => {
                let (_read_half, write_half) = tokio::io::split(conn);
                return Some(write_half);
            }
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < 20 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(_) => return None,
        }
    }
}
