//! What a server's response frame carries for a version request.

use std::str::{self, Utf8Error};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What the provider says it is.
///
/// One of these on channel `0`, then the scope finishes.
///
/// The version is the whole payload — no tag, because there is nothing
/// to discriminate. It runs to the end, so it needs no length either.
///
/// It may be empty, which is a provider declining to say. That is an
/// answer, and one a caller can act on, rather than the absence of one.
///
/// # It is a string, and this layer does not read it
///
/// No number, no three fields, no ordering. What a version MEANS is
/// between the two ends: a semantic version, a build hash, a date, a
/// name. A shape imposed here would be this specification deciding how
/// providers are allowed to version themselves, which is not its to
/// decide and not something it could revise once decided.
///
/// So comparing two of them is a caller's business. A caller that
/// wants to know whether a provider is new enough knows what its own
/// versions look like; nothing here can help it and nothing here will
/// get in the way.
///
/// # There is no failure
///
/// Alone among the responses in this specification. Every other scope
/// can come back with the provider saying it could not — an image it
/// cannot supply, a container that would not start — because every
/// other scope asks it to DO something.
///
/// This asks it to say what it is, which it always knows. A provider
/// that could not answer this could not have received the question.
///
/// Which is also why there is nothing to tag. A tag tells two things
/// apart, and there are not two things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The version, borrowed from the frame it arrived in.
    pub &'a str,
);

/// The string's own bytes, and nothing in front of them.
impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): a string's bytes are
    /// already bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// One way to fail: bytes that are not UTF-8. No bytes at all is
    /// the empty version, which is a provider declining to say rather
    /// than a frame that went wrong.
    type Error = Utf8Error;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        str::from_utf8(bytes).map(Frame)
    }
}
