//! Writing a type into a frame's payload bytes.

use std::fmt;
use std::io;

/// Write `Self` as the payload of one frame.
///
/// By reference, and into a buffer the caller owns. Both matter, and
/// for the same reason: a payload is never sent alone. It goes out
/// behind a nine-byte header, so a caller writes the header into a
/// buffer and then asks the payload to append itself — one allocation
/// per frame, and no copy to join the two halves.
///
/// There is no variant that returns a fresh `Vec`. It would be the
/// wrong call at every site in this protocol, since none of them want
/// a payload without the header in front of it, and a convenience
/// nobody should reach for is worse than none at all. Anything that
/// genuinely wants the bytes alone — hashing one, writing one to
/// disk — can hand over an empty buffer.
///
/// See [`Decode`](crate::decode::Decode) for why the format lives in
/// the type rather than in the caller.
pub trait Encode {
    /// What went wrong, in the format's own words.
    ///
    /// An associated type for the same reason
    /// [`Decode::Error`](crate::decode::Decode::Error) is one: there
    /// is no single format here, so there is no single failure, and
    /// erasing it would cost a caller the only useful thing about it.
    ///
    /// A payload that cannot fail to encode names [`Infallible`].
    ///
    /// [`Infallible`]: std::convert::Infallible
    type Error: std::error::Error + Send + Sync + 'static;

    /// Append this payload to `out`.
    ///
    /// An implementation that fails partway may leave bytes behind.
    /// The caller knows how long the buffer was before it handed the
    /// [`Writer`] over, so recovering is a truncate — but it has to be
    /// done rather than assumed.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error>;
}

/// Where a payload writes itself.
///
/// A buffer that can only be APPENDED to. The header is already in it
/// by the time an implementation gets here, and this type is what
/// makes overwriting it impossible rather than merely discouraged:
/// the `Vec` is private, and nothing on the public surface can reach
/// a byte that was already there. No indexing, no `clear`, no
/// `truncate`, no `as_mut_slice`.
///
/// It implements [`io::Write`], so the serializers hand off to it
/// directly — `serde_json::to_writer`, `ciborium::into_writer` — and
/// those can only move forward too. There is nothing to seek with.
///
/// A `&mut [u8]` would have given the same guarantee and cost too
/// much: a payload's encoded length is not known until it is encoded,
/// so a fixed slice would make every implementation size itself in
/// advance, twice, and agree with itself both times.
pub struct Writer<'a> {
    buffer: &'a mut Vec<u8>,
    start: usize,
}

impl<'a> Writer<'a> {
    /// Start writing a payload at the end of `buffer`.
    ///
    /// Whatever is in it stays — this appends.
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        let start = buffer.len();
        Self { buffer, start }
    }

    /// Append bytes.
    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Reserve capacity for `additional` more bytes.
    ///
    /// Capacity only. Nothing readable changes, and neither does
    /// [`len`](Self::len).
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }

    /// How many bytes this writer has appended.
    ///
    /// The payload's length so far — not the buffer's, which still
    /// counts the header in front of it.
    pub fn len(&self) -> usize {
        self.buffer.len() - self.start
    }

    /// Whether nothing has been appended yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl io::Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buffer.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Debug for Writer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Writer").field("len", &self.len()).finish()
    }
}
