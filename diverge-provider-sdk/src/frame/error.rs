//! Why a frame could not be decoded.

/// The bytes a header occupies: `type` plus `scope` plus `channel`.
///
/// A constant, which is the point of a fixed header — the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
pub const HEADER_LEN: usize = 1 + 4 + 4;

/// A frame that could not be read.
///
/// Both are about the ENVELOPE, and there is no third. Every payload is
/// bytes here — including an auth frame's, which this layer used to
/// parse — so the only things that can go wrong are a header too short
/// to read and a type nobody defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Fewer than [`HEADER_LEN`] bytes.
    Truncated,
    /// A `type` this layer does not define.
    ///
    /// Every frame kind is fixed and enumerated, in both directions,
    /// so an unfamiliar value is a malformed frame rather than a peer
    /// with more protocol than this one. The blanks count too: a
    /// client sending `2` through `4` is sending scope-level replies
    /// only a server has, and a server sending `1` is opening a scope
    /// it has no business opening.
    ///
    /// What a newer peer legitimately has more of is request KINDS,
    /// and those are discriminated inside payloads rather than by this
    /// byte — so growth never widens the type space and never has to
    /// be tolerated here.
    UnknownType(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown frame type {byte}")
            }
        }
    }
}

/// No source. Neither variant wraps another error, because neither is
/// about a payload — what a payload's own decoder makes of it is
/// reported by that decoder, in its own vocabulary.
impl std::error::Error for FrameError {}

/// Split a frame's header off the front, returning
/// `(type, scope, channel, payload)`.
///
/// Shared by both directions: the header is the same bytes whoever
/// sent it, and only the MEANING of `type` differs.
pub(super) fn split_header(
    bytes: &[u8],
) -> Result<(u8, u32, u32, &[u8]), FrameError> {
    let header: &[u8; HEADER_LEN] = bytes
        .get(..HEADER_LEN)
        .and_then(|head| head.try_into().ok())
        .ok_or(FrameError::Truncated)?;
    let r#type = header[0];
    let scope = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let channel =
        u32::from_be_bytes([header[5], header[6], header[7], header[8]]);
    Ok((r#type, scope, channel, &bytes[HEADER_LEN..]))
}
