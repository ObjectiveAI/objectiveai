//! What a FUSE serve could not do.

use std::fmt;

use super::super::super::server::response;
use crate::wire::client::handle::SendError;
use crate::container_proxy::outside::endpoints::fuse::mount::server::channel_request::FrameEncodeError;
use crate::wire::frame;
use crate::shared::containers::response::VolumeMode;
use crate::shared::error::Error;

/// A FUSE serve that did not open, or was not answered.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request could not be sent.
    Send(SendError),
    /// The request did not serialize.
    Request(postcard::Error),
    /// The connection went away before the provider answered.
    Closed,
    /// A frame that would not decode.
    Frame(frame::FrameError),
    /// The provider finished the scope with nothing before the finish:
    /// it could not serve the request, and said nothing else.
    Unanswered,
    /// A frame that cannot be the first on a main stream.
    Misrouted,
    /// The provider's answer did not decode.
    Response(response::FrameError),
    /// The provider refused to serve: a running container has the
    /// volume, and the provider does not FUSE serve a volume of its
    /// mode beside one.
    Mounted,
    /// The provider refused to serve: another FUSE serve holds the
    /// volume, and the provider FUSE serves a volume of its mode once.
    FuseServed,
    /// The provider refused to serve: another operation has the
    /// volume to itself.
    Held,
    /// The provider refused to serve: the volume is not in the mode
    /// the request means it to have, and this is the mode it is in.
    VolumeMode(VolumeMode),
    /// The provider refused to serve: it does not FUSE serve a volume
    /// of the mode the volume is in.
    Unsupported,
    /// The provider refused to serve: a volume the caller cannot see,
    /// one it could not open.
    Refused(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "volume FUSE serve request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "volume FUSE serve request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("the connection ended before the volume FUSE serve was answered"),
            ExecuteError::Frame(error) => write!(f, "frame did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the provider could not serve the volume"),
            ExecuteError::Misrouted => f.write_str("a frame that cannot open a FUSE serve answered it"),
            ExecuteError::Response(error) => write!(f, "{error}"),
            ExecuteError::Mounted => f.write_str("a running container has the volume, and the provider does not FUSE serve a volume of its mode beside one"),
            ExecuteError::FuseServed => f.write_str("another FUSE serve holds the volume, and the provider FUSE serves a volume of its mode once"),
            ExecuteError::Held => f.write_str("another operation has the volume to itself"),
            ExecuteError::VolumeMode(refused) => {
                write!(f, "the volume `{}` is in mode `{:?}`, not the mode the FUSE serve means", refused.name, refused.mode)
            }
            ExecuteError::Unsupported => f.write_str("the provider does not FUSE serve a volume of the mode the volume is in"),
            ExecuteError::Refused(_) => f.write_str("the provider refused to serve the volume"),
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
            | ExecuteError::Refused(_) => None,
        }
    }
}

/// An ask that has no answer.
#[derive(Debug)]
pub enum AskError {
    /// The ask did not encode: a path too long for its prefix.
    Encode(FrameEncodeError),
    /// The ask could not be sent: the scope, or the connection, is
    /// gone.
    Send(SendError),
    /// The provider finished the channel with nothing before the
    /// finish: it could not serve the ask.
    Unserved,
    /// The connection went away with the ask open.
    Closed,
}

impl fmt::Display for AskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AskError::Encode(error) => write!(f, "volume FUSE serve ask did not encode: {error}"),
            AskError::Send(error) => write!(f, "volume FUSE serve ask could not be sent: {error}"),
            AskError::Unserved => f.write_str("the provider could not serve the ask"),
            AskError::Closed => f.write_str("the connection ended with the ask open"),
        }
    }
}

impl std::error::Error for AskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AskError::Encode(error) => Some(error),
            AskError::Send(error) => Some(error),
            AskError::Unserved | AskError::Closed => None,
        }
    }
}
