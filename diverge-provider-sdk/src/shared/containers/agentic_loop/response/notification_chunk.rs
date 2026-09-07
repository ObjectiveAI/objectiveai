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
/// [`is_fatal`](Self::is_fatal) says which. A caller that only cares
/// whether the run survived reads one boolean; a caller that wants the
/// whole commentary reads every one of these and decides for itself.
///
/// The alternative was two chunk variants with the same three fields,
/// which would have made "the loop warned me" and "the loop failed"
/// different shapes to parse rather than different values in one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: NotificationChunkType,
    /// Whether the run ends here.
    ///
    /// `true` is the loop saying it is over and this is why. `false`
    /// is it saying something and carrying on — a warning, a retry, a
    /// degraded mode, a failure it recovered from.
    ///
    /// Fatal rather than merely wrong, because "wrong" is not a
    /// question a caller can act on and "over" is. A provider that
    /// hits an error and retries past it has not failed, and a caller
    /// told otherwise would abandon a run that was still going.
    ///
    /// Nothing here says what a provider must send either way. What is
    /// worth mentioning, and what it can recover from, is the
    /// provider's to decide.
    pub is_fatal: bool,
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
