//! What a server's response frame carries for a version request.

use std::str::{self, Utf8Error};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What the provider says it is: the revision of this specification
/// it implements.
///
/// One of these on channel `0`, then the scope finishes.
///
/// The revision is the whole payload — no tag, because there is
/// nothing to discriminate. It runs to the end, so it needs no length
/// either.
///
/// # The string is fixed, and the specification defines it
///
/// The revision of the specification the provider implements, which
/// is this crate's version: `MAJOR.MINOR.PATCH`, the string every
/// page of the specification prints as its revision. The handler
/// sends the crate's own, read from the manifest at compile time.
/// Nothing else is a conforming answer — not a build hash, not a
/// name, and not the empty string. A caller reads it as the one fact
/// it needs before composing anything else: which specification the
/// far end speaks.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a>(
    /// The revision, borrowed from the frame it arrived in.
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
    /// One way to fail: bytes that are not UTF-8. What the string
    /// says is the caller's to judge against the revision it expects.
    type Error = Utf8Error;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        str::from_utf8(bytes).map(Frame)
    }
}
