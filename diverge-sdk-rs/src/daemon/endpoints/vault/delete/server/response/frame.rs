//! What a server's response frame carries for a delete.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A delete's answer: the key or the value is gone, no such key or
/// slot, a record binds the key, a container holds the value, the
/// value is the key's last, forbidden, or a failure.
///
/// A delete is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Deleted`](Self::Deleted),
/// `1` for [`NotFound`](Self::NotFound), `2` for [`InUse`](Self::InUse),
/// `3` for [`Locked`](Self::Locked), `4` for [`Last`](Self::Last),
/// `5` for [`Forbidden`](Self::Forbidden), `6` for
/// [`Error`](Self::Error) — and only the error carries anything after
/// it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`InUse`](Self::InUse),
/// [`Locked`](Self::Locked) and [`Last`](Self::Last) are ANSWERS:
/// the daemon looked, and either no key has the name or the key no
/// such slot; or the key is there and an agent or a tool binds it —
/// a fact of their records, which outlasts any run; or a running
/// container holds the value, or one of the key's — a fact of the
/// run, gone at its inactive; or the slot is the key's only one,
/// which a delete of the key removes — and in every case nothing
/// changed and nothing is retried. An [`Error`](Self::Error) is the
/// absence of an answer: the daemon could not delete, for whatever
/// reason it knows, and the key is as it was.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing
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
    /// The key, or the value named, is gone. Tag `0`.
    Deleted,
    /// No key has the name, or the key has no slot of the number;
    /// nothing changed. Tag `1`.
    NotFound,
    /// An agent or a tool binds the key in its `secrets`, and it was
    /// left as it is; nothing changed. Tag `2`.
    InUse,
    /// A running container holds the value named — or, for a whole
    /// key, one of its values — as a `rotating` secret, and it was
    /// left as it is; nothing changed. Tag `3`. Wait for the
    /// container to go inactive, and ask again.
    Locked,
    /// The slot named is the key's only one, and it was left as it
    /// is; nothing changed. Tag `4`. A key holds at least one value:
    /// delete the key instead.
    Last,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `5`.
    Forbidden,
    /// A failure. Tag `6`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 2;

/// Tag for [`Frame::Locked`].
const LOCKED: u8 = 3;

/// Tag for [`Frame::Last`].
const LAST: u8 = 4;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 5;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 6;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
                Ok(())
            }
            Frame::Locked => {
                out.extend_from_slice(&[LOCKED]);
                Ok(())
            }
            Frame::Last => {
                out.extend_from_slice(&[LAST]);
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
            DELETED => Ok(Frame::Deleted),
            NOT_FOUND => Ok(Frame::NotFound),
            IN_USE => Ok(Frame::InUse),
            LOCKED => Ok(Frame::Locked),
            LAST => Ok(Frame::Last),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A delete response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("vault delete response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown vault delete response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "vault delete error did not parse: {error}"),
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
