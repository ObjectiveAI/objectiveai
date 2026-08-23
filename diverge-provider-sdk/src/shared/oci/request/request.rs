//! One registry request, as the runtime wrote it.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// One request against the caller's registry, verbatim.
///
/// A request line and headers, exactly as the container runtime put
/// them on the socket. Nothing here reads them.
///
/// # Whole, in one frame
///
/// Because a pull has no request body. `GET` fetches a manifest or a
/// blob, `HEAD` probes for one, and everything that qualifies the
/// ask — `Range`, `Accept` — is a header. So a request ends at the
/// blank line and fits in whatever a frame will carry.
///
/// A push would not fit and is not possible: a blob is arbitrarily
/// large, and there is no second frame to put the rest in. That is a
/// property of this shape rather than a rule imposed on top of it.
///
/// # Borrowed
///
/// From the frame it arrived in, and forwarded from there. A relay that
/// copied would be copying a request it never reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Request<'a>(
    /// The bytes.
    pub &'a [u8],
);

/// Straight through. There is no encoding step because there is
/// nothing encoded — a request arrives as bytes and leaves as the same
/// bytes.
impl Encode for Request<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode, and saying so is better than inventing an error nobody can
    /// produce and every caller has to handle.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

impl<'a> Decode<'a> for Request<'a> {
    /// [`Infallible`]: every byte string is one of these.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        Ok(Request(bytes))
    }
}
