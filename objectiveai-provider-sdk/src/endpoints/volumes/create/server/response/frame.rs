//! What a server's response frame carries for a volume creation.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The volume exists.
///
/// One of these on channel `0`, then the scope finishes. It carries
/// nothing, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | a [`Frame`], then a finish | the volume exists and a listing will show it |
/// | a finish, and nothing before it | it does not, and nothing partial does |
/// | nothing | the connection died; whether it was made is unknowable from here |
///
/// # Why it does not answer with the volume
///
/// Because the caller already knows both fields. It chose the
/// [`name`](crate::endpoints::volumes::create::client::request::Frame::name),
/// and a
/// [`created`](crate::endpoints::volumes::list::server::response::Volume::created)
/// it can predict to the second is not news. Sending a
/// [`Volume`](crate::endpoints::volumes::list::server::response::Volume)
/// back would be echoing a request with a timestamp stapled to it, and
/// a caller that wants the canonical record asks for a
/// [`list`](crate::endpoints::volumes::list) — which is the same
/// answer every other caller gets, rather than a second version of the
/// truth minted here.
///
/// # The scope ends, and the volume does not
///
/// Unlike a [`laboratory`](crate::endpoints::laboratories::create),
/// whose scope IS the container's life. A volume outlives the request
/// that made it and every connection the caller ever holds; it goes
/// away when a
/// [`delete`](crate::endpoints::volumes::delete) says so and not
/// before.
///
/// Which is what makes it worth having. A caller mounts one into a
/// laboratory, the laboratory stops, and the work is still there.
///
/// # It still spends a byte
///
/// A payload of zero bytes would carry the same information today and
/// cost a wire break tomorrow. Failure has no shape yet — a provider
/// says so by finishing without this — and when it gets one it will be
/// another tag value, which is only additive if there is a tag to add
/// to.
///
/// The same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says the volume exists.
const CREATED: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[CREATED]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&CREATED) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A volume creation result that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a provider reporting a failure will send, once
    /// failures have a shape. Until then it is a peer that disagrees
    /// about the protocol.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("volume creation result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume creation result frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
