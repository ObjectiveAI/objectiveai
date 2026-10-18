//! What a server's response frame carries for a set.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A set's answer: the key holds the value, no such key or slot,
/// the slot is locked, forbidden, or a failure.
///
/// A set is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Set`](Self::Set), `1` for
/// [`NotFound`](Self::NotFound), `2` for [`Locked`](Self::Locked),
/// `3` for [`Forbidden`](Self::Forbidden), `4` for
/// [`Error`](Self::Error) — and only the error carries anything after
/// it.
///
/// # Answers, and one failure
///
/// [`Set`](Self::Set) is the ANSWER, whether the key was made, the
/// value appended or a slot's value replaced: the three are one
/// request, and a caller that wants to know which gets first.
/// [`NotFound`](Self::NotFound) and [`Locked`](Self::Locked) are
/// answers too: the daemon looked, and either the slot named — or
/// the key it was named on — is not there, or it is and a running
/// container holds it, and in either case nothing changed and
/// nothing is retried. An [`Error`](Self::Error) is the absence of an
/// answer: the daemon could not set the value, for whatever reason it
/// knows, and the key is as it was.
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
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"found":…}`, `{"error":…}` — and
/// a bare variant the string alone — `"not_found"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The key holds the value. Tag `0`.
    Set,
    /// The slot named is not there — no key has the name, or the key
    /// has no slot of the number; nothing changed. Tag `1`.
    NotFound,
    /// A running container holds the slot named as a `rotating`
    /// secret, and it was left as it is; nothing changed. Tag `2`.
    /// Wait for the container to go inactive, or name another slot.
    Locked,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Set`].
const SET: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Locked`].
const LOCKED: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Set => {
                out.extend_from_slice(&[SET]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Locked => {
                out.extend_from_slice(&[LOCKED]);
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
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            SET => Ok(Frame::Set),
            NOT_FOUND => Ok(Frame::NotFound),
            LOCKED => Ok(Frame::Locked),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A set response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("vault set response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown vault set response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "vault set error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
