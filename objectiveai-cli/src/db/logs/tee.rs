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

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(unix)]
use interprocess::local_socket::GenericFilePath;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::Name;
use interprocess::local_socket::tokio::prelude::*;
use objectiveai_sdk::cli::websocket_agents_instances_listener::{
    ConversationRow, RowContent, RowTableKind,
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
    /// A fully-resolved conversation row.
    Row { row: ConversationRow },
    /// A consumed message-queue notification. The writer only knows
    /// the `message_queue_contents.id` — the content and the parent
    /// queue row's metadata live in the DB, so the DAEMON resolves
    /// them before fan-out (notifications are low-frequency).
    MessageQueueContent {
        agent_instance_hierarchy: String,
        response_id: String,
        message_queue_content_id: i64,
        /// RFC3339 — when the writer shipped this frame.
        delivered_at: String,
    },
}

/// Build the owned tee frame for one row the writer is about to
/// write. `created_at` is the same timestamp the SQL will bind.
/// Metadata mapping: HEAD rows carry their payload (`tool_call_id` /
/// choice `key`) on the row's metadata fields with `RowContent::Head`;
/// tool-call rows carry `tool_call_id` both inline (content) and as
/// metadata, mirroring the snapshot reader's joined shape.
pub fn row_to_frame(value: &RowValue<'_>, created_at: i64) -> TeeFrame {
    use RowContent as C;
    use RowTableKind as K;
    let delivered_at = crate::db::time::unix_to_rfc3339(created_at);
    if let RowValue::MessageQueueContent {
        response_id,
        agent_instance_hierarchy,
        message_queue_content_id,
    } = value
    {
        return TeeFrame::MessageQueueContent {
            agent_instance_hierarchy: (*agent_instance_hierarchy).to_string(),
            response_id: (*response_id).to_string(),
            message_queue_content_id: *message_queue_content_id,
            delivered_at,
        };
    }
    let (table, content, tool_call_id, choice_key) = match value {
        RowValue::MessageQueueContent { .. } => {
            unreachable!("early-returned above")
        }
        // ---- head rows (metadata carriers; no content payload) ----
        RowValue::ToolResponse { tool_call_id, .. } => (
            K::ToolResponse,
            C::Head,
            Some((*tool_call_id).to_string()),
            None,
        ),
        RowValue::RequestMessageTool { tool_call_id, .. } => (
            K::RequestMessageTool,
            C::Head,
            Some((*tool_call_id).to_string()),
            None,
        ),
        RowValue::RequestVectorChoice { key, .. } => (
            K::RequestVectorChoice,
            C::Head,
            None,
            Some((*key).to_string()),
        ),
        // ---- assistant response ----
        RowValue::AssistantResponseRefusal { text, .. } => (
            K::AssistantResponseRefusal,
            C::Refusal { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::AssistantResponseReasoning { text, .. } => (
            K::AssistantResponseReasoning,
            C::Reasoning { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::AssistantResponseToolCalls {
            tool_call_id,
            function_name,
            arguments,
            ..
        } => (
            K::AssistantResponseToolCalls,
            C::ToolCall {
                tool_call_id: (*tool_call_id).to_string(),
                function_name: (*function_name).to_string(),
                arguments: (*arguments).to_string(),
            },
            Some((*tool_call_id).to_string()),
            None,
        ),
        RowValue::AssistantResponseContentText { text, .. } => (
            K::AssistantResponseContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::AssistantResponseContentImage { image_url, .. } => (
            K::AssistantResponseContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::AssistantResponseContentAudio { input_audio, .. } => (
            K::AssistantResponseContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::AssistantResponseContentVideo { video_url, .. } => (
            K::AssistantResponseContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::AssistantResponseContentFile { file, .. } => (
            K::AssistantResponseContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- tool response content ----
        RowValue::ToolResponseContentText { text, .. } => (
            K::ToolResponseContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::ToolResponseContentImage { image_url, .. } => (
            K::ToolResponseContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::ToolResponseContentAudio { input_audio, .. } => (
            K::ToolResponseContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::ToolResponseContentVideo { video_url, .. } => (
            K::ToolResponseContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::ToolResponseContentFile { file, .. } => (
            K::ToolResponseContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- request message: user ----
        RowValue::RequestMessageUserContentText { text, .. } => (
            K::RequestMessageUserContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestMessageUserContentImage { image_url, .. } => (
            K::RequestMessageUserContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageUserContentAudio { input_audio, .. } => (
            K::RequestMessageUserContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageUserContentVideo { video_url, .. } => (
            K::RequestMessageUserContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageUserContentFile { file, .. } => (
            K::RequestMessageUserContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- request message: assistant ----
        RowValue::RequestMessageAssistantRefusal { text, .. } => (
            K::RequestMessageAssistantRefusal,
            C::Refusal { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestMessageAssistantReasoning { text, .. } => (
            K::RequestMessageAssistantReasoning,
            C::Reasoning { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestMessageAssistantToolCalls {
            tool_call_id,
            function_name,
            arguments,
            ..
        } => (
            K::RequestMessageAssistantToolCalls,
            C::ToolCall {
                tool_call_id: (*tool_call_id).to_string(),
                function_name: (*function_name).to_string(),
                arguments: (*arguments).to_string(),
            },
            Some((*tool_call_id).to_string()),
            None,
        ),
        RowValue::RequestMessageAssistantContentText { text, .. } => (
            K::RequestMessageAssistantContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestMessageAssistantContentImage { image_url, .. } => (
            K::RequestMessageAssistantContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageAssistantContentAudio { input_audio, .. } => (
            K::RequestMessageAssistantContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageAssistantContentVideo { video_url, .. } => (
            K::RequestMessageAssistantContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageAssistantContentFile { file, .. } => (
            K::RequestMessageAssistantContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- request message: tool content ----
        RowValue::RequestMessageToolContentText { text, .. } => (
            K::RequestMessageToolContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestMessageToolContentImage { image_url, .. } => (
            K::RequestMessageToolContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageToolContentAudio { input_audio, .. } => (
            K::RequestMessageToolContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageToolContentVideo { video_url, .. } => (
            K::RequestMessageToolContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::RequestMessageToolContentFile { file, .. } => (
            K::RequestMessageToolContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- vector request choice content ----
        RowValue::RequestVectorChoiceContentText { text, .. } => (
            K::RequestVectorChoiceContentText,
            C::Text { text: (*text).to_string() },
            None,
            None,
        ),
        RowValue::RequestVectorChoiceContentImage { image_url, .. } => (
            K::RequestVectorChoiceContentImage,
            C::Image((*image_url).clone()),
            None,
            None,
        ),
        RowValue::RequestVectorChoiceContentAudio { input_audio, .. } => (
            K::RequestVectorChoiceContentAudio,
            C::Audio((*input_audio).clone()),
            None,
            None,
        ),
        RowValue::RequestVectorChoiceContentVideo { video_url, .. } => (
            K::RequestVectorChoiceContentVideo,
            C::Video((*video_url).clone()),
            None,
            None,
        ),
        RowValue::RequestVectorChoiceContentFile { file, .. } => (
            K::RequestVectorChoiceContentFile,
            C::File((*file).clone()),
            None,
            None,
        ),
        // ---- vector response vote ----
        RowValue::ResponseVectorVote { vote, .. } => (
            K::ResponseVectorVote,
            C::Vote { vote: vote.to_vec() },
            None,
            None,
        ),
    };
    TeeFrame::Row {
        row: ConversationRow {
            agent_instance_hierarchy: value.agent_instance_hierarchy().to_string(),
            response_id: value.response_id().to_string(),
            table,
            row_index: value.row_index(),
            row_sub_index: value.row_sub_index(),
            delivered_at,
            tool_call_id,
            choice_key,
            sender_agent_instance_hierarchy: None,
            queued_at: None,
            message_queue_key: None,
            message_queue_id: None,
            content,
        },
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
