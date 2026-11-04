//! What a server's response frame carries for an identities subjects.

use serde::Serialize;
use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A subjects' answer: the subjects, forbidden, or a failure.
///
/// One question and one reply, so there is exactly one of these per
/// scope, before the finish that ends it. A payload leads with one byte
/// saying which — `0` for [`Listed`](Self::Listed), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: the array of strings for the
/// first, nothing for the second, the error for the third.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing what the request
/// asks, and nothing changed. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"listed":[…]}`, `{"error":…}` — and
/// a bare variant the string alone — `"forbidden"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Every subject signed in at the issuer, in order; none where no
    /// identity is signed in there. Tag `0`, then the JSON array of
    /// them.
    Listed(Vec<String>),
    /// The account the request is served for holds no grant allowing
    /// it; nothing was read. Tag `1`.
    Forbidden,
    /// A failure: the identities could not be read. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answer cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Listed(items) => {
                out.extend_from_slice(&[LISTED]);
                serde_json::to_writer(out, items)
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
            LISTED => serde_json::from_slice(rest).map(Frame::Listed).map_err(FrameError::Listed),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An identities subjects response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The subjects did not parse.
    Listed(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("identities subjects response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown identities subjects response frame tag {tag}"),
            FrameError::Listed(error) => write!(f, "identities subjects did not parse: {error}"),
            FrameError::Error(error) => write!(f, "identities subjects error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Listed(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
