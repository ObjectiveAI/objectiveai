//! What a server's response frame carries for an attach.

use serde::Serialize;
use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An attach's answer: the tool is attached to the template, no tool or
/// no template is the one named, or a failure.
///
/// An attach is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Attached`](Self::Attached),
/// `1` for [`NoTool`](Self::NoTool), `2` for
/// [`NoTemplate`](Self::NoTemplate), `3` for [`Forbidden`](Self::Forbidden),
/// `4` for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Two answers, one failure
///
/// [`NoTool`](Self::NoTool) and [`NoTemplate`](Self::NoTemplate) are
/// ANSWERS: the daemon looked, and one of the two names is nobody's,
/// so nothing was changed and nothing is retried — the caller has a
/// wrong name. An [`Error`](Self::Error) is the absence of an answer:
/// the daemon could not attach the tool, for whatever reason it
/// knows, and the attachment is as it was.
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
    /// The tool is attached to the template. Tag `0`.
    Attached,
    /// No tool of the caller's has the tool's name; nothing was
    /// changed. Tag `1`.
    NoTool,
    /// No agent template of the daemon's has the id; nothing was
    /// changed. Tag `2`.
    NoTemplate,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// The tool is attached to the template if it was, and not if it
    /// was not. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Attached`].
const ATTACHED: u8 = 0;

/// Tag for [`Frame::NoTool`].
const NO_TOOL: u8 = 1;

/// Tag for [`Frame::NoTemplate`].
const NO_TEMPLATE: u8 = 2;

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
            Frame::Attached => {
                out.extend_from_slice(&[ATTACHED]);
                Ok(())
            }
            Frame::NoTool => {
                out.extend_from_slice(&[NO_TOOL]);
                Ok(())
            }
            Frame::NoTemplate => {
                out.extend_from_slice(&[NO_TEMPLATE]);
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
            ATTACHED => Ok(Frame::Attached),
            NO_TOOL => Ok(Frame::NoTool),
            NO_TEMPLATE => Ok(Frame::NoTemplate),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An attach response frame that could not be read.
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
            FrameError::Empty => f.write_str("standalone tools attach agent template response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown standalone tools attach agent template response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "standalone tools attach agent template error did not parse: {error}"),
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
