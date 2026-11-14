//! What a volume FUSE serve could not do.

use std::fmt;

use super::super::super::server::response;
use crate::shared::containers::response::VolumeMode;
use crate::shared::error::Error;
use crate::wire::client::handle::SendError;
use crate::wire::frame;

/// A FUSE serve that did not open: the machinery's failures, and each
/// answer of the daemon's that is not `Serving`.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before the answer.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// The daemon finished the scope with no answer.
    Unanswered,
    /// A frame that cannot answer a request.
    Misrouted,
    /// An answer that would not decode.
    Response(response::FrameError),
    /// A running container has the volume.
    Mounted,
    /// Another FUSE serve holds the volume.
    FuseServed,
    /// Another operation has the volume to itself.
    Held,
    /// The volume is in another mode.
    VolumeMode(VolumeMode),
    /// The provider does not serve a volume of that mode.
    Unsupported,
    /// The caller's grants do not allow it.
    Forbidden,
    /// No such provider, or no such volume on it.
    NotFound,
    /// The daemon's error, or the provider's in its own words.
    Daemon(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "volumes fuse serve request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "volumes fuse serve request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the connection ended before the volumes fuse serve was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the daemon could not serve the volume"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a fuse serve answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Mounted => f.write_str("a running container has the volume"),
            ExecuteError::FuseServed => f.write_str("another fuse serve holds the volume"),
            ExecuteError::Held => f.write_str("another operation has the volume to itself"),
            ExecuteError::VolumeMode(refused) => {
                write!(f, "the volume `{}` is in mode `{:?}`, not the mode the fuse serve means", refused.name, refused.mode)
            }
            ExecuteError::Unsupported => f.write_str("the provider does not fuse serve a volume of that mode"),
            ExecuteError::Forbidden => f.write_str("the account may not fuse serve the volume"),
            ExecuteError::NotFound => f.write_str("the daemon knows no such provider, or the provider lists no such volume"),
            ExecuteError::Daemon(error) => write!(f, "the daemon would not serve the volume: {}", error.0),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Misrouted
            | ExecuteError::Mounted
            | ExecuteError::FuseServed
            | ExecuteError::Held
            | ExecuteError::VolumeMode(_)
            | ExecuteError::Unsupported
            | ExecuteError::Forbidden
            | ExecuteError::NotFound
            | ExecuteError::Daemon(_) => None,
        }
    }
}
