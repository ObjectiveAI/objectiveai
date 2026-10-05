//! What a server's response frame carries for an edit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An edit's answer: the account is as the request states, no account
/// is the one named, no such role, no such hook resource, the name or
/// the identity is another's, the change would leave the account alone,
/// forbidden, or a failure.
///
/// An edit is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Edited`](Self::Edited), `1`
/// for [`NotFound`](Self::NotFound), `2` for [`NoRole`](Self::NoRole),
/// `3` for [`NoResource`](Self::NoResource), `4` for
/// [`InUse`](Self::InUse), `5` for [`Alone`](Self::Alone), `6` for
/// [`Forbidden`](Self::Forbidden), `7` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`NoRole`](Self::NoRole),
/// [`NoResource`](Self::NoResource), [`InUse`](Self::InUse) and
/// [`Alone`](Self::Alone) are ANSWERS: the daemon looked, and no
/// account is the one named, or a role named is none it has, or the
/// hook names no resource the caller holds or not one that is a hook,
/// or the name or the identity requested is another account's, or the
/// change would leave the account with neither a name nor a credential,
/// or would take the name from an account a container runs under or the
/// credential from one a client is connected as, and in each case
/// nothing changed and nothing is retried. An [`Error`](Self::Error) is
/// the absence of an answer: the daemon could not make the change, for
/// whatever reason it knows, and the account is as it was. The request
/// is applied whole or not at all.
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
    /// The account is as the request states. Tag `0`.
    Edited,
    /// No account is the one named; nothing changed. Tag `1`.
    NotFound,
    /// A role named is none the daemon has; nothing changed. Tag `2`.
    NoRole,
    /// The hook names no resource the caller holds, or one that is not
    /// a directory with `hook.yaml` at its root; nothing changed. Tag
    /// `3`.
    NoResource,
    /// The name or the identity requested is another account's; nothing
    /// changed. Tag `4`.
    InUse,
    /// The change would leave the account with neither a name nor a
    /// credential, or take the name from one a container runs under, or
    /// the credential from one a client is connected as; nothing
    /// changed. Tag `5`.
    Alone,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `6`.
    Forbidden,
    /// A failure. Tag `7`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::NoRole`].
const NO_ROLE: u8 = 2;

/// Tag for [`Frame::NoResource`].
const NO_RESOURCE: u8 = 3;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 4;

/// Tag for [`Frame::Alone`].
const ALONE: u8 = 5;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 6;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 7;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Edited => {
                out.extend_from_slice(&[EDITED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::NoRole => {
                out.extend_from_slice(&[NO_ROLE]);
                Ok(())
            }
            Frame::NoResource => {
                out.extend_from_slice(&[NO_RESOURCE]);
                Ok(())
            }
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
                Ok(())
            }
            Frame::Alone => {
                out.extend_from_slice(&[ALONE]);
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
            EDITED => Ok(Frame::Edited),
            NOT_FOUND => Ok(Frame::NotFound),
            NO_ROLE => Ok(Frame::NoRole),
            NO_RESOURCE => Ok(Frame::NoResource),
            IN_USE => Ok(Frame::InUse),
            ALONE => Ok(Frame::Alone),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An edit response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's eight.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("accounts edit response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown accounts edit response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "accounts edit error did not parse: {error}"),
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
