//! Why a frame on the `/mcp/list-tools` path could not be decoded.

use crate::shared::mcp;

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
/// nobody defines — and the rest are the payloads, which this path
/// types on both sides.
#[derive(Debug)]
pub enum FrameError {
    /// Fewer bytes than a header — a container frame with no channel
    /// byte, or a server frame shorter than [`HEADER_LEN`].
    Truncated,
    /// A `type` this path does not define, on a server frame — the
    /// one direction that has a type byte to be wrong about.
    UnknownType(u8),
    /// A container request whose params would not parse.
    Request(serde_json::Error),
    /// A server response that would not decode as this exchange's.
    Response(mcp::FrameError),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("list-tools frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown list-tools frame type {byte}")
            }
            FrameError::Request(error) => {
                write!(f, "list-tools request did not parse: {error}")
            }
            FrameError::Response(error) => {
                write!(f, "list-tools response did not decode: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Request(error) => Some(error),
            FrameError::Response(error) => Some(error),
            FrameError::Truncated | FrameError::UnknownType(_) => None,
        }
    }
}

/// Split a server frame's header off the front, returning
/// `(type, channel, payload)`.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u8, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    Ok((header[0], header[1], &bytes[HEADER_LEN..]))
}
