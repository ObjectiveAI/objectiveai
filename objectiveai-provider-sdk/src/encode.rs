//! Writing a type into a frame's payload bytes.

use std::error::Error;
use std::fmt;

/// Write `Self` as the payload of one frame.
///
/// By reference, and into a buffer the caller owns. Both matter, and
/// for the same reason: a payload is never sent alone. It goes out
/// behind a nine-byte header, so a caller writes the header into a
/// buffer and then asks the payload to append itself — one allocation
/// per frame, and no copy to join the two halves. Returning a fresh
/// `Vec` would force both.
///
/// [`encode`](Self::encode) is provided for when there is nothing to
/// append to.
///
/// See [`Decode`](crate::decode::Decode) for why the format lives in
/// the type rather than in the caller.
pub trait Encode {
    /// Append this payload to `out`.
    ///
    /// Whatever is already in `out` is left alone — a caller that has
    /// written a header is appending to it, not replacing it.
    ///
    /// An implementation that fails partway may leave bytes behind. A
    /// caller that intends to recover should truncate `out` back to
    /// the length it had before the call rather than assume nothing
    /// was written.
    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError>;

    /// Encode into a fresh buffer.
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut out = Vec::new();
        self.encode_into(&mut out)?;
        Ok(out)
    }
}

/// A payload that could not be written.
///
/// Opaque for the same reason [`DecodeError`](crate::decode::DecodeError)
/// is: naming the format's error type here would put the format back
/// into a signature that exists to keep it out. The cause is reachable
/// through [`Error::source`].
///
/// Rarer than a decode failure, and not impossible — a map with
/// non-string keys, a float where the format admits none, or an
/// implementation writing to something that can fail.
pub struct EncodeError(Box<dyn Error + Send + Sync>);

impl EncodeError {
    /// Wrap the format's error.
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}

impl fmt::Debug for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "payload did not encode: {}", self.0)
    }
}

impl Error for EncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
