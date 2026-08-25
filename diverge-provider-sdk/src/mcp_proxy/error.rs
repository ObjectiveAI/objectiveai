//! Why a proxy frame could not be decoded.

/// The bytes a header occupies: `type` plus `channel`.
///
/// A constant, for the same reason the main protocol's
/// [`HEADER_LEN`](crate::frame::HEADER_LEN) is one: the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
pub const HEADER_LEN: usize = 1 + 1;

/// A frame that could not be read.
///
/// Both are about the ENVELOPE, and there is no third: every payload
/// is bytes here, so the only things that can go wrong are a header
/// too short to read and a type nobody defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Fewer than [`HEADER_LEN`] bytes.
    Truncated,
    /// A `type` this wire does not define.
    ///
    /// Every frame kind is fixed and enumerated, in both directions,
    /// so an unfamiliar value is a malformed frame rather than a peer
    /// with more protocol than this one. The directions count too: a
    /// provider sending `0` is opening a channel only the proxy
    /// mints, and a proxy sending `1` or `2` is answering a request
    /// nobody made.
    UnknownType(u8),
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
        }
    }
}

/// No source. Neither variant wraps another error, because neither is
/// about a payload — what a payload's own decoder makes of it is
/// reported by that decoder, in its own vocabulary.
impl std::error::Error for FrameError {}

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
