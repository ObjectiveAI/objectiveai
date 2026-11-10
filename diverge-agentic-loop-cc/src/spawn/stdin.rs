//! What the harness writes to Claude Code's stdin — the shapes, and
//! how a line gets written.
//!
//! The mirror of [`response`](crate::response), pointed the other
//! way and holding only what this container actually says — the
//! stream-json INPUT vocabulary is far wider (the source's whole
//! stdin union), but a shape nothing sends is a shape nobody has had
//! to be right about. Serialize-only, the way `response` is
//! deserialize-only: each type becomes one NDJSON line and nothing
//! here ever reads one back.

use std::io;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::process;

/// A user message for the running session: an enqueued message, or
/// the run's first one — stream-json input mode's way of delivering
/// the print prompt is the same line an enqueue writes.
///
/// The [`uuid`](Self::uuid) is the correlation key: with
/// `--replay-user-messages` on, Claude Code echoes the message back
/// on stdout at the moment it enters the conversation, carrying this
/// uuid — which is how the reader finds the batch whose user parts
/// the echo marks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessage {
    /// Always `user`.
    pub r#type: UserMessageType,
    /// The message body.
    pub message: UserMessageBody,
    /// The correlation key the replay echo carries back.
    pub uuid: String,
}

/// A [`UserMessage`]'s body: role and content, nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessageBody {
    /// Always `user`.
    pub role: UserRole,
    /// The message's blocks, in the Anthropic API's own shape — what
    /// [`content::blocks`](super::content::blocks) made of the
    /// message's MCP content.
    pub content: Vec<Block>,
}

/// One block of a user message, in the Anthropic API's shape: the
/// two kinds this container's conversion produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Text.
    Text {
        /// The text itself.
        text: String,
    },
    /// An image, by its bytes.
    Image {
        /// Where the bytes are.
        source: ImageSource,
    },
}

/// An image's bytes, inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64, with its format.
    Base64 {
        /// `image/jpeg`, `image/png`, `image/gif` or `image/webp`.
        media_type: String,
        /// The bytes, base64.
        data: String,
    },
}

/// The `user` type literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserMessageType {
    /// The only value.
    #[default]
    User,
}

/// The `user` role literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// The only value.
    #[default]
    User,
}

/// One enqueued (or initial) message, as its NDJSON line.
pub fn user_message_line(blocks: Vec<Block>, uuid: String) -> String {
    let mut line = serde_json::to_string(&UserMessage {
        r#type: Default::default(),
        message: UserMessageBody {
            role: Default::default(),
            content: blocks,
        },
        uuid,
    })
    .expect("a stdin line is plain structs and serializes");
    line.push('\n');
    line
}

/// Write already-newline-terminated lines, then flush once.
pub async fn write_lines(
    child_stdin: &mut process::ChildStdin,
    lines: &str,
) -> io::Result<()> {
    child_stdin.write_all(lines.as_bytes()).await?;
    child_stdin.flush().await
}
