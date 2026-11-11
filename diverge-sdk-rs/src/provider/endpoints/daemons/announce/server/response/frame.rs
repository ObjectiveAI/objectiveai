//! What a server's response frame carries for an announce.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The refusal of a second announce on one connection.
///
/// An announce is a scope kept open, and an announce that stands is
/// answered by nothing. A payload is one byte, `0` for
/// [`AlreadyOpen`](Self::AlreadyOpen). The rules:
///
/// 1. **No response while it stands.** A provider that serves the
///    announce sends no response for it. The scope is open, and that is
///    the whole answer.
/// 2. **Listed while open.** While the scope is open, the caller's
///    identity is in the provider's
///    [`daemons::list`](crate::provider::endpoints::daemons::list), with
///    the request's announcement: sent as `Added` to every listing open
///    then, and part of the snapshot of every listing opened after.
/// 3. **The end.** The scope ends on the caller's cancel, or on the
///    caller's connection ending, and on nothing else. The caller then
///    leaves the listing — `Removed`, to every listing open — and after
///    a cancel the finish follows.
/// 4. **One per connection.** A connection with an announce open
///    already is sent `AlreadyOpen` first and alone, and the finish
///    follows it — see
///    [the endpoints](crate::provider::endpoints#one-open-at-a-time).
///    The announce already open is not changed by it. A caller that
///    changes its announcement cancels the scope and opens another.
/// 5. **Independent.** Announcing is independent of listening, of
///    sending messages and of accepting: an identity announces without
///    a [`daemons::listen`](crate::provider::endpoints::daemons::listen)
///    or a [`daemons::accept`](crate::provider::endpoints::daemons::accept)
///    open, holds either without announcing, and sends a
///    [`daemons::message`](crate::provider::endpoints::daemons::message)
///    either way.
/// 6. **No error.** Every provider serves the announce, and a request
///    that decodes is a well-formed announcement, so no response fails.
///
/// # A finish with nothing is an answer
///
/// The caller cancelled, or the request was not served — a request
/// whose announcement is malformed among them: a scope that finishes
/// with no response before it is that, not a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// Refused: this connection has an announce open already. Tag `0`.
    ///
    /// Carries nothing. Sent first and alone (rule 4).
    AlreadyOpen,
}

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 0;

/// The tag alone.
impl Encode for Frame {
    /// Nothing can fail: the frame is one byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::AlreadyOpen => {
                out.extend_from_slice(&[ALREADY_OPEN]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail: nothing, or a tag that is not the one.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An announce response that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("announce response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown announce response frame tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
