//! What a client's channel request frame carries on a serve scope.

use std::fmt;

use crate::container_proxy::outside::fuse::mount::server::channel_request as mount;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a client asks the provider for while a volume is served: one
/// of the nine asks, or the stop.
///
/// A payload leads with one byte saying which. The nine asks carry
/// the tags they carry on a mount's scope — `0` read, `1` write, `2`
/// list, `3` remove, `4` rename, `5` mkdir, `6` stat, `7` truncate,
/// `8` setattr — and are that scope's very frames, so a bridge
/// forwards bytes; `9` is the stop, bare.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0`–`8` | [`Ask`](Self::Ask): the mount's ask of that tag |
/// | `9` | [`Stop`](Self::Stop) |
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One of the nine, answered on its channel with that ask's own
    /// frame — the shared vocabulary's — and the finish. The provider
    /// answers as a caller's server would, from the volume.
    Ask(mount::Frame<'a>),
    /// Stop serving. Tag `9`.
    ///
    /// Carries nothing, and nothing comes back on this channel: the
    /// provider sends no channel response and no channel response
    /// finish on it. What comes back is the scope's finish, after
    /// every ask still being answered has been: the provider gives
    /// the volume back and finishes. A second stop changes nothing.
    Stop,
}

/// Tag for [`Frame::Stop`].
const STOP: u8 = 9;

impl Encode for Frame<'_> {
    /// The ask's own failure; the stop cannot fail.
    type Error = mount::FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Ask(ask) => ask.encode(out),
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// The ask's own failures, and the empty payload.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            None => Err(FrameError::Empty),
            Some(&STOP) => Ok(Frame::Stop),
            Some(_) => mount::Frame::decode(bytes).map(Frame::Ask).map_err(FrameError::Ask),
        }
    }
}

/// A serve channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// The ask did not decode: an unknown tag, or fewer bytes than it
    /// promises.
    Ask(mount::FrameError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volume serve channel request frame is empty"),
            FrameError::Ask(error) => write!(f, "volume serve ask did not decode: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Ask(error) => Some(error),
            FrameError::Empty => None,
        }
    }
}
