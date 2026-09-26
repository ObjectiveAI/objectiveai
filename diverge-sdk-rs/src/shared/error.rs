//! What went wrong, as far as this specification describes it.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An error, carrying one JSON value and nothing else.
///
/// This specification does not say what is inside it. Not "not yet" —
/// it says nothing, and a reader should not expect a shape to appear
/// here later.
///
/// # Why a value rather than a vocabulary
///
/// Because the alternative is this crate tracking every failure every
/// provider can have. A typed enum would have to name what a container
/// runtime, a kernel, a filesystem, a registry and an upstream model
/// can each go wrong with, and would be wrong the first time any of
/// them added one. What a caller does about most of them is the same
/// thing anyway — stop, or try again later — and that decision does
/// not need the protocol to have a word for the cause.
///
/// So a provider puts in whatever it knows. A caller that understands
/// a particular provider reads it; one that does not still has
/// something to log, surface, or hand to a person, which is more than
/// a bare failure gives it.
///
/// # It is JSON, and that is not a free choice
///
/// [`serde_json::Value`] deserializes through `deserialize_any`, which
/// a format with no self-description cannot answer. So this decodes
/// from JSON and not from [`postcard`], whatever module it ends up
/// riding in — a postcard payload that wanted to carry one would have
/// to tunnel it through a string, and would be inventing a second
/// encoding for a value that already has one.
///
/// # It is a message, not a Rust error
///
/// Deliberately no [`std::error::Error`] impl. This is a thing that
/// arrives on a wire; the `Error` types elsewhere in this crate are
/// what a decoder returns when it cannot read one. Giving both the
/// same trait would blur two ideas that share only a name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Error(
    /// Whatever the provider had to say.
    ///
    /// [`Value::Null`](serde_json::Value::Null) is legal and is what
    /// [`Default`] gives — an error whose sender had nothing to add.
    pub serde_json::Value,
);

impl Encode for Error {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Error {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
