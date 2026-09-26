//! A write's content, arriving.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A piece of the file being written.
///
/// Bytes and nothing else — no tag, because there is nothing here to
/// discriminate. This is the CONTENT, and content is the same content
/// wherever a write happens.
///
/// # Failing is not this type's business
///
/// Whether a channel carrying this can also carry a failure is the
/// endpoint's to decide, and the endpoints that allow one wrap this in
/// an enum of their own — every
/// [`containers`](crate::provider::endpoints::containers) scope does.
///
/// Putting the failure here would mean one vocabulary of errors for
/// every endpoint that ever streams a write, decided by whichever
/// needed one first. What can go wrong is endpoint logic; what a piece
/// of a file looks like is not.
///
/// # How it ends
///
/// With a finish, and only with a finish. There is no terminator in
/// the payload because the frame layer already has one, and a second
/// would be two signals for one fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame<'a>(
    /// The bytes, borrowed from the frame they arrived in.
    pub &'a [u8],
);

impl Encode for Frame<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// [`Infallible`]: there is nothing to get wrong about a slice
    /// that is already the answer.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Frame(bytes))
    }
}
