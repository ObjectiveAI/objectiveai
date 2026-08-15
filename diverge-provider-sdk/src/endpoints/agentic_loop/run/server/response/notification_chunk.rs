//! The notification chunk.

use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

/// Something the loop has to say that is not part of its output.
///
/// A warning, a retry, a degraded mode, a failure — anything a
/// provider wants a caller to know about the run itself rather than
/// about what the agent produced.
///
/// In-band rather than a transport signal, because a loop keeps going
/// after most of these and can fail after producing output. Ending the
/// stream without saying why would leave a caller holding a partial
/// result and no way to tell it apart from a complete one.
///
/// # Whether it is fatal is a field, not a type
///
/// [`is_error`](Self::is_error) says which. A caller that only cares
/// about failures reads one boolean; a caller that wants the whole
/// commentary reads every one of these and decides for itself.
///
/// The alternative was two chunk variants with the same three fields,
/// which would have made "the loop warned me" and "the loop failed"
/// different shapes to parse rather than different values in one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: NotificationChunkType,
    /// Whether this is a failure.
    ///
    /// `true` is the loop reporting that something went wrong; `false`
    /// is it reporting anything else. Nothing here says what a
    /// provider must send either as — what is worth mentioning, and
    /// what counts as failing, is the provider's to decide.
    pub is_error: bool,
    /// The message or details, as an arbitrary JSON value — providers
    /// report failures in shapes we do not get to dictate, and
    /// flattening one into a string would discard the structure a
    /// caller needs to act on it.
    pub message: serde_json::Value,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    ///
    /// Same key and same type as the chunks that flatten rmcp types
    /// carry, so a trace id attached to a content chunk can be
    /// attached here too — these three are ours rather than MCP's, but
    /// that is no reason for them to be the one place a trace stops.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// [`NotificationChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NotificationChunkType {
    #[serde(rename = "notification")]
    #[default]
    Notification,
}
