//! What a server's response frame carries for an admit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;


/// An admit's answer: the admission is made, with its key or without
/// one; no tool is the one named; the tool is somebody else's; an
/// admission for the identity is there already; or a failure.
///
/// An admit is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Admitted`](Self::Admitted),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Connected`](Self::Connected), `3` for [`Exists`](Self::Exists),
/// `4` for [`Forbidden`](Self::Forbidden), `5` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON:
/// the key for the first, nothing for the bare answers, the error for
/// the last.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`Connected`](Self::Connected) and
/// [`Exists`](Self::Exists) are ANSWERS: the daemon looked, and no tool
/// of the caller's is the one named, or it is a connected tool whose
/// runner alone admits, or an admission for that identity is on the
/// tool already, and in each case nothing was made and nothing is
/// retried. An [`Error`](Self::Error) is the absence of an answer: the
/// daemon could not make the admission, for whatever reason it knows,
/// and the tool is as it was.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's, or the
/// `account` of the container it came from — holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The admission is on the tool. The key the daemon minted when the
    /// admission admits a connect, answered here and never again;
    /// `null` for one that admits a list alone. Tag `0`.
    Admitted(Option<String>),
    /// No tool of the caller's is the one named; nothing changed. Tag
    /// `1`.
    NotFound,
    /// The tool is a connected one, somebody else's: its runner admits,
    /// and the daemon cannot; nothing changed. Tag `2`.
    Connected,
    /// An admission for that identity is on the tool already; nothing
    /// changed. Take it back and admit again to change it. Tag `3`.
    Exists,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `4`.
    Forbidden,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Admitted`].
const ADMITTED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Connected`].
const CONNECTED: u8 = 2;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 3;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Admitted(key) => {
                out.extend_from_slice(&[ADMITTED]);
                serde_json::to_writer(out, key)
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Connected => {
                out.extend_from_slice(&[CONNECTED]);
                Ok(())
            }
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
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
            ADMITTED => serde_json::from_slice(rest).map(Frame::Admitted).map_err(FrameError::Admitted),
            NOT_FOUND => Ok(Frame::NotFound),
            CONNECTED => Ok(Frame::Connected),
            EXISTS => Ok(Frame::Exists),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An admit response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The key did not parse.
    Admitted(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools admit response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools admit response frame tag {tag}"),
            FrameError::Admitted(error) => write!(f, "tools admit key did not parse: {error}"),
            FrameError::Error(error) => write!(f, "tools admit error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Admitted(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
