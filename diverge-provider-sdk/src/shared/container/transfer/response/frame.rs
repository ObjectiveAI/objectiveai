//! What a response frame carries on a transfer channel.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The file landed.
///
/// It carries nothing, because saying so IS the whole message — no
/// fields, and no bytes on the wire either.
///
/// Which is [`write_path`](crate::shared::container::write_path)'s
/// shape, and a separate type rather than an alias of it. They mean
/// the same thing today and will not once either grows a field: a
/// transfer knows things about a destination in another container that
/// a write into this one has no room for.
///
/// # Failing is not this type's business
///
/// It used to spend a tag byte against the day failure got a shape.
/// Failure has one now, and it is not here: the endpoints that allow a
/// transfer to fail wrap this in an enum of their own —
/// [`laboratories::run`](crate::endpoints::laboratories::run::server::channel_response::transfer::Frame)
/// and
/// [`laboratories::connect`](crate::endpoints::laboratories::connect::server::channel_response::transfer::Frame)
/// both do — and that enum's tag does the discriminating.
///
/// Which is also where a transfer's own failures belong. A source that
/// is not there, or two containers a provider will not put in contact,
/// are things this exchange can go wrong with and a write into a
/// container cannot.
///
/// # A silent tear is possible here
///
/// A file being copied can be written underneath the copy, and a
/// provider can DETECT it — `fstat` before and after — without being
/// able to prevent it. Nothing distinguishes a transfer that copied a
/// moving file from one that copied a still one.
///
/// So a transfer that returns this frame promises the destination
/// exists and holds one whole file. It does not promise that file ever
/// existed at the source. [`read`](crate::shared::container::read) is
/// in the same position, for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

impl Encode for Frame {
    /// [`Infallible`]: writing nothing has no failure mode.
    type Error = Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// [`Infallible`]: there is nothing to read.
    type Error = Infallible;

    /// Whatever bytes arrive are ignored. There are none to send, so a
    /// peer that sent some knows something this version does not, and
    /// leaving room for it is cheaper than refusing it.
    fn decode(_bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Frame)
    }
}
