//! What a client's response frame carries on an authorize channel.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Yes or no.
///
/// The whole answer to an
/// [`Authorize`](crate::endpoints::laboratories::create::server::channel_request::Frame::Authorize),
/// and one frame is all there is — this is not a stream, and a channel
/// carrying one of these finishes immediately after.
///
/// # Why it says nothing else
///
/// A reason would have to mean something to the provider, and the
/// provider did not write the question. What was asked is between the
/// two ends; the only part this layer needs is whether the answer was
/// yes, because that is the part a provider acts on.
///
/// A caller that wants to explain itself has somewhere better to do it
/// than a channel whose whole purpose is to unblock something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame(
    /// Whether it is authorized.
    pub bool,
);

/// The byte for `false`.
const DENIED: u8 = 0;

/// The byte for `true`.
const AUTHORIZED: u8 = 1;

/// One byte, and no serialization.
///
/// A bool has two states and a byte has room for them, so a format
/// would be a format applied to nothing. The two values are named
/// rather than written as literals because `0` for no and `1` for yes
/// is a convention, not a law, and a reader should not have to assume
/// which way round it went.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): writing one known
    /// byte has no failure mode.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[if self.0 { AUTHORIZED } else { DENIED }]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&DENIED) => Ok(Frame(false)),
            Some(&AUTHORIZED) => Ok(Frame(true)),
            Some(&byte) => Err(FrameError::Unknown(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// An authorization answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all.
    Empty,
    /// A byte that is neither yes nor no.
    ///
    /// Rejected rather than read as truthy. Anything other than the
    /// two defined values means the sender and this reader disagree
    /// about the protocol, and guessing which way a disagreement
    /// leans is a poor way to decide an authorization.
    Unknown(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("authorization answer frame is empty")
            }
            FrameError::Unknown(byte) => {
                write!(f, "authorization answer is neither {DENIED} nor {AUTHORIZED}: {byte}")
            }
        }
    }
}

impl Error for FrameError {}
