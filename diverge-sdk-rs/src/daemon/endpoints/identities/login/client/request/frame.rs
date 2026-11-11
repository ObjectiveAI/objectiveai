//! What a client's request frame carries for an identities login.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::shared::relay::Relay;

/// Ask the daemon to sign an identity in on its host, as `diverge
/// identities login` signs one in on the host it runs on: by its
/// password where one is given — OpenID Connect's Authorization Code
/// login, the password sent as Basic credentials and the redirect read,
/// no browser — else by the device authorization grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The issuer, exactly as its discovery document names it.
    pub issuer: String,
    /// The subject to sign in as: the `sub` the approval must name.
    pub subject: String,
    /// The public client the daemon signs in as at the issuer.
    pub client_id: String,
    /// The scope asked for, space-separated; `openid` and
    /// `offline_access` are added where it lacks them. Absent, those two
    /// alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// What the access token is for, RFC 8707's `resource`. Absent, the
    /// issuer's own choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// The identity's password: the login is the Authorization Code
    /// login with no browser, the password the authorization request's
    /// Basic credentials. Absent, the device authorization grant.
    /// The daemon's command log keeps it redacted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. See the table in
/// [`endpoints`](crate::daemon::endpoints) for the whole allocation.
const TAG: u8 = 128;

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

/// An identities login request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("identities login request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected identities login request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "identities login request did not parse: {error}"),
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
