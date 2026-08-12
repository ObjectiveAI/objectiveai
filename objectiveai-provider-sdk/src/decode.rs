//! Reading a type out of a frame's payload bytes.

use std::error::Error;
use std::fmt;

/// Read `Self` from the bytes of one frame's payload.
///
/// Deliberately narrower than [`serde::Deserialize`]. A Deserialize
/// impl says how a type maps onto SOME data format, and leaves the
/// choice of format to whoever calls it. This says which format, and
/// says it once, in the type — so a payload has exactly one wire form
/// and no caller can pick a different one by accident.
///
/// That is what lets the crate use more than one format without
/// anything having to track which is which. MCP payloads are JSON
/// because that channel relays JSON-RPC and must hand it on
/// byte-identical; a filetree event has no such obligation and can be
/// binary. Both are just types with a `Decode` impl, and the frame
/// layer calls the same method on either.
///
/// # The lifetime
///
/// `Self` may borrow from `bytes` — [`mcp::Request`] holds its
/// JSON-RPC body as a `&RawValue` pointing into the frame it arrived
/// in, and the tunnel payloads are `&[u8]` outright. A payload that
/// owns everything implements `Decode<'_>` and ignores it.
///
/// [`serde::Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
/// [`mcp::Request`]: crate::agentic_loop::server::request::mcp::Request
pub trait Decode<'a>: Sized {
    /// Read one payload.
    ///
    /// `bytes` is the payload alone — the frame header has already
    /// been split off, so this never sees a type, a scope or a
    /// channel, and cannot be confused about where the payload starts.
    fn decode(bytes: &'a [u8]) -> Result<Self, DecodeError>;
}

/// A payload that could not be read.
///
/// Opaque, and carrying whatever the format's own error was. Naming
/// the underlying error type here would put the format back in the
/// signature, which is the thing [`Decode`] exists to keep out of it —
/// a caller that matched on a `serde_json::Error` would break the day
/// a payload moved to CBOR, having done nothing wrong.
///
/// The cause is still reachable through [`Error::source`] for anyone
/// debugging rather than branching.
pub struct DecodeError(Box<dyn Error + Send + Sync>);

impl DecodeError {
    /// Wrap the format's error.
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "payload did not decode: {}", self.0)
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
