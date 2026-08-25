//! Why a proxy frame could not be decoded.

use crate::endpoints::agentic_loop::run::server::channel_request;

/// The bytes a SERVER frame's header occupies: `type` plus
/// `channel`.
///
/// A constant, for the same reason the main protocol's
/// [`HEADER_LEN`](crate::frame::HEADER_LEN) is one: the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
///
/// The container's frame is not counted here: it has one kind, so no
/// type byte, and its header is the channel alone.
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
    /// A `type` this wire does not define, on a server frame — the
    /// one direction that has a type byte to be wrong about.
    ///
    /// Every frame kind is fixed and enumerated, so an unfamiliar
    /// value is a malformed frame rather than a peer with more
    /// protocol than this one.
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

/// Split a server frame's header off the front, returning
/// `(type, channel, payload)`.
///
/// The server's alone: a container frame has no type byte, and its
/// decoder splits the channel off itself.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u8, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    Ok((header[0], header[1], &bytes[HEADER_LEN..]))
}
