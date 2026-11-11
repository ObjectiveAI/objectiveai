//! What a server's response frame carries for a disconnect.

use serde::Serialize;
use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A disconnect's answer: how many connections were closed, no account
/// is the one named, forbidden, or a failure.
///
/// One question and one reply, so there is exactly one of these per
/// scope, before the finish that ends it. A payload leads with one byte
/// saying which — `0` for [`Disconnected`](Self::Disconnected), `1` for
/// [`NotFound`](Self::NotFound), `2` for [`Forbidden`](Self::Forbidden),
/// `3` for [`Error`](Self::Error) — and `Disconnected` and the error
/// carry JSON after it.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing what the request asks
/// over what it names, and nothing changed. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"disconnected":1}`, `{"error":…}` —
/// and a bare variant the string alone — `"not_found"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Every connection a client had as it was told to close: how many,
    /// which is `0` where none was open. Tag `0`.
    Disconnected(u64),
    /// No account, and no administrator, is the one named; nothing
    /// changed. Tag `1`.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Disconnected`].
const DISCONNECTED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and the variant's own JSON where it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Disconnected(count) => {
                out.extend_from_slice(&[DISCONNECTED]);
                serde_json::to_writer(out, count)
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DISCONNECTED => serde_json::from_slice(rest).map(Frame::Disconnected).map_err(FrameError::Count),
            NOT_FOUND => Ok(Frame::NotFound),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A disconnect response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The count did not parse.
    Count(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("accounts disconnect response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown accounts disconnect response frame tag {tag}"),
            FrameError::Count(error) => write!(f, "accounts disconnect count did not parse: {error}"),
            FrameError::Error(error) => write!(f, "accounts disconnect error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Count(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
