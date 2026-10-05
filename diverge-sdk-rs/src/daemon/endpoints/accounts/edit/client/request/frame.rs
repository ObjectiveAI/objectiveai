//! What a client's request frame carries for an edit.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::edit::Change;
use crate::daemon::endpoints::accounts::{Credential, Reference};
/// Ask the daemon to change an account. Every member but the reference
/// is an optional [`Change`]: absent leaves it as it is, `"delete"`
/// takes it away, `{"set":…}` replaces it whole — the roles the new
/// list entire, the credential a new credential, of either form. A
/// request with every member absent changes nothing and is not a
/// failure.
///
/// # Never neither
///
/// An account has a name, a credential, or both. A request that deletes
/// the name of an account with no credential, or the credential of an
/// account with no name, or both of one with both, is refused as
/// leaving the account alone, and nothing changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The account to change: see [`Reference`].
    pub account: Reference,
    /// The name, unique among accounts; another's is the edit's
    /// `InUse`. Absent, as it is; `delete`, the account has no name and
    /// dials in alone, which a container running under it forbids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Change<String>>,
    /// How a client dials in as the account: see [`Credential`].
    /// Replaced whole, which is how a key rotates or a hook moves; a
    /// key naming an identity another account's key names is the edit's
    /// `InUse`. Absent, as it is; `delete`, no client dials in as it
    /// from then on, which a client connected as it now forbids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<Change<Credential>>,
    /// What the account is for, in words. Absent, as it is; `delete`,
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Change<String>>,
    /// The roles it holds, by name: the new list whole, each one the
    /// daemon has and the caller holds the `grant` grant over. Absent,
    /// as they are; `delete`, none, and the account may do nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Change<Vec<String>>>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them —
/// [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the
/// whole allocation. The values are chosen across modules that do not
/// know about each other, so the table is the only place they can be
/// seen at once.
const TAG: u8 = 52;

/// JSON, as every request of the daemon's is.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A accounts edit request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("accounts edit request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected accounts edit request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "accounts edit request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
