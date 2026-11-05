//! What a server's response frame carries for an identities login.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::shared::Identity;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A login's answers: the code a person approves, then how the login
/// ended; or why it did not begin.
///
/// A device login begun is exactly one [`Pending`](Self::Pending), then
/// exactly one of [`SignedIn`](Self::SignedIn),
/// [`Denied`](Self::Denied), [`Expired`](Self::Expired),
/// [`OtherSubject`](Self::OtherSubject) or [`Error`](Self::Error), then
/// the finish. A password login is exactly one of `SignedIn`,
/// [`Rejected`](Self::Rejected), `OtherSubject` or `Error`, with no
/// `Pending`, then the finish. A login refused is exactly one of
/// [`Held`](Self::Held), [`Forbidden`](Self::Forbidden) or
/// [`Error`](Self::Error), then the finish. A payload leads with one
/// byte saying which — `0` to `8`, in the order of the variants — and
/// the rest is that variant's own JSON, or nothing for a bare one. No
/// answer carries a token or a password.
///
/// # A finish with nothing after `Pending`
///
/// The client cancelled, or its connection ended, before the person
/// decided: the login ended, and nothing was written.
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
/// snake case, over its payload — `{"pending":…}`, `{"signed_in":…}` —
/// and a bare variant the string alone — `"denied"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The login begun: what a person needs to approve it. Tag `0`,
    /// once, first.
    Pending(Pending),
    /// Approved as the identity named, and written on the daemon's host,
    /// replacing one already there. Tag `1`.
    SignedIn(Identity),
    /// Another login of the identity, or a reader of it, holds its lock:
    /// nothing was asked of the issuer. Tag `2`.
    Held,
    /// The person denied the code; nothing was written. Tag `3`.
    Denied,
    /// The code expired before the person decided; nothing was written.
    /// Tag `4`.
    Expired,
    /// Approved as another identity, the one carried; nothing was
    /// written. Tag `5`.
    OtherSubject(Identity),
    /// The account the request is served for holds no grant allowing
    /// it; nothing was asked of the issuer. Tag `6`.
    Forbidden,
    /// A failure: the issuer could not be reached, refused, or the
    /// identity could not be written. Tag `7`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
    /// The issuer refused the subject and the password; nothing was
    /// written. Tag `8`.
    Rejected,
}

/// What a person needs to approve a login, as the issuer's device
/// authorization answer gives it: never the device code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Pending {
    /// The code the person enters.
    pub user_code: String,
    /// Where the person enters it.
    pub verification_uri: String,
    /// Where the person need not enter it: the URI with the code in it,
    /// where the issuer gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    /// The seconds the code is good for.
    pub expires_in: u64,
}

const PENDING: u8 = 0;
const SIGNED_IN: u8 = 1;
const HELD: u8 = 2;
const DENIED: u8 = 3;
const EXPIRED: u8 = 4;
const OTHER_SUBJECT: u8 = 5;
const FORBIDDEN: u8 = 6;
const ERROR: u8 = 7;
const REJECTED: u8 = 8;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Pending(pending) => {
                out.extend_from_slice(&[PENDING]);
                serde_json::to_writer(out, pending)
            }
            Frame::SignedIn(identity) => {
                out.extend_from_slice(&[SIGNED_IN]);
                serde_json::to_writer(out, identity)
            }
            Frame::Held => {
                out.extend_from_slice(&[HELD]);
                Ok(())
            }
            Frame::Denied => {
                out.extend_from_slice(&[DENIED]);
                Ok(())
            }
            Frame::Expired => {
                out.extend_from_slice(&[EXPIRED]);
                Ok(())
            }
            Frame::OtherSubject(identity) => {
                out.extend_from_slice(&[OTHER_SUBJECT]);
                serde_json::to_writer(out, identity)
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::Rejected => {
                out.extend_from_slice(&[REJECTED]);
                Ok(())
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
            PENDING => serde_json::from_slice(rest).map(Frame::Pending).map_err(FrameError::Body),
            SIGNED_IN => serde_json::from_slice(rest).map(Frame::SignedIn).map_err(FrameError::Body),
            HELD => Ok(Frame::Held),
            DENIED => Ok(Frame::Denied),
            EXPIRED => Ok(Frame::Expired),
            OTHER_SUBJECT => serde_json::from_slice(rest).map(Frame::OtherSubject).map_err(FrameError::Body),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            REJECTED => Ok(Frame::Rejected),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A login response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    UnknownTag(u8),
    /// A variant's own JSON did not parse.
    Body(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("identities login response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown identities login response frame tag {tag}"),
            FrameError::Body(error) => write!(f, "identities login answer did not parse: {error}"),
            FrameError::Error(error) => write!(f, "identities login error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
