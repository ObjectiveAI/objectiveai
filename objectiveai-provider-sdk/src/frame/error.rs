//! Why a frame could not be decoded.

/// The bytes a header occupies: `type` plus `scope` plus `channel`.
///
/// A constant, which is the point of a fixed header — the payload
/// starts at a known offset rather than wherever a parse happened to
/// finish.
pub const HEADER_LEN: usize = 1 + 4 + 4;

/// A frame that could not be read.
///
/// Not [`Clone`], [`PartialEq`] or [`Copy`], which the other types
/// here are: [`Malformed`](Self::Malformed) carries a
/// [`serde_json::Error`], and that is none of them. Keeping the parse
/// failure is worth more than the derives — a rejected request is
/// something someone has to debug, and `expected value at line 1
/// column 84` is the whole of what they need.
#[derive(Debug)]
pub enum FrameError {
    /// Fewer than [`HEADER_LEN`] bytes.
    Truncated,
    /// A `type` no frame in this direction can have.
    ///
    /// Only a CLIENT frame can produce this. A client's types are a
    /// closed set — `0` through `5` — so a sixth value is malformed.
    /// A server's are open above `4`, since each is a kind of request
    /// this layer does not interpret, so an unfamiliar one is a
    /// request from a newer peer rather than an error.
    UnknownType(u8),
    /// A payload this layer DOES interpret, that did not parse.
    ///
    /// Only a client request can produce this, and for the same reason
    /// [`UnknownType`](Self::UnknownType) is one-sided: the client's
    /// requests are a closed set, so this layer knows their types and
    /// decodes them. Everything else it carries — bodies, credentials,
    /// server requests — it hands on as bytes and cannot judge.
    Malformed(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated => {
                f.write_str("frame is shorter than its header")
            }
            FrameError::UnknownType(byte) => {
                write!(f, "unknown client frame type {byte}")
            }
            FrameError::Malformed(error) => {
                write!(f, "client request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Malformed(error) => Some(error),
            FrameError::Truncated | FrameError::UnknownType(_) => None,
        }
    }
}

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
