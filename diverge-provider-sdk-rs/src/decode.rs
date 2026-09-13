//! Reading a type out of a frame's payload bytes.

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
/// `Self` may borrow from `bytes` — a [`blob`] is a `&[u8]` pointing
/// into the frame it arrived in, and so is everything else this crate
/// relays without reading. A payload that owns
/// everything implements `Decode<'_>` and ignores it.
///
/// [`serde::Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
/// [`blob`]: crate::shared::containers::oci::blob::response::Frame
pub trait Decode<'a>: Sized {
    /// What went wrong, in the format's own words.
    ///
    /// An associated type rather than one error for the whole crate,
    /// because there is no one format here and so no one failure. A
    /// JSON payload fails with `serde_json::Error`, a CBOR one with
    /// CBOR's — and a caller that knows which payload it is reading
    /// gets that error whole, with its line and column, rather than
    /// something erased on the way out.
    ///
    /// Generic code names [`Self::Error`] and stays out of it.
    ///
    /// `Send + Sync + 'static` so this survives being carried: across
    /// an await, between tasks, and into a `Box<dyn Error>` where one
    /// error has to stand for several — which is what the frame layer
    /// does, since a single `FrameError` covers payload types that do
    /// not share a format.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Read one payload.
    ///
    /// `bytes` is the payload alone — the frame header has already
    /// been split off, so this never sees a type, a scope or a
    /// channel, and cannot be confused about where the payload starts.
    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error>;
}
