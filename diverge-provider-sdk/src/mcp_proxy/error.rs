//! Why a proxy frame could not be decoded.

use crate::endpoints::agentic_loop::run::server::channel_request;

/// The bytes a header occupies: `type` plus `channel`.
///
/// A constant, for the same reason the main protocol's
/// [`HEADER_LEN`](crate::frame::HEADER_LEN) is one: the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
pub const HEADER_LEN: usize = 1 + 1;

/// A frame that could not be read.
///
/// Two are about the envelope — a header too short to read, a type
/// nobody defines — and the third is the one payload this wire types:
/// a container's request that would not decode. A server response's
/// payload is bytes to this layer, so it has no failure to report
/// here; what an opener's own decoder makes of one is reported by
/// that decoder, in its own vocabulary.
#[derive(Debug)]
pub enum FrameError {
    /// Fewer than [`HEADER_LEN`] bytes.
    Truncated,
    /// A `type` this wire does not define.
    ///
    /// Every frame kind is fixed and enumerated, in both directions,
    /// so an unfamiliar value is a malformed frame rather than a peer
    /// with more protocol than this one. The directions count too: a
    /// server sending `0` is opening a channel only the container
    /// mints, and a container sending `1` or `2` is answering a
    /// request nobody made.
    UnknownType(u8),
    /// A container request whose exchange would not decode.
    Request(channel_request::FrameError),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("proxy frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown proxy frame type {byte}")
            }
            FrameError::Request(error) => {
                write!(f, "a proxy channel request could not be read: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Request(error) => Some(error),
            FrameError::Truncated | FrameError::UnknownType(_) => None,
        }
    }
}

/// Split a frame's header off the front, returning
/// `(type, channel, payload)`.
///
/// Shared by both directions: the header is the same bytes whoever
/// sent it, and only the MEANING of `type` differs.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u8, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    Ok((header[0], header[1], &bytes[HEADER_LEN..]))
}
