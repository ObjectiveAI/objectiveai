//! What a response frame carries on a write path channel.

use std::convert::Infallible;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The file landed.
///
/// It carries nothing, because saying so IS the whole message — no
/// fields, and no bytes on the wire either.
///
/// # Failing is not this type's business
///
/// It used to spend a tag byte against the day failure got a shape.
/// Failure has one now, and it is not here: the endpoints that allow a
/// write to fail wrap this in an enum of their own — every
/// [`containers`](crate::provider::endpoints::containers) scope does — and that
/// enum's tag does the discriminating. A byte here
/// as well would be two discriminators for one choice.
///
/// What a write can fail with is the exchange's business. What "the
/// file landed" looks like is not, and it looks like nothing.
///
/// # What a partial write leaves behind
///
/// Nothing at the destination. A provider writes to a temporary in the
/// destination's own directory and renames it into place, so the path
/// holds the old file, then nothing, then the new one — never a prefix
/// of the new one. That holds whether this frame arrives or not.
///
/// Where space is too tight for both copies, unlinking the old one
/// first frees exactly what the new one needs. That trades the old
/// contents away on failure, which is why it is worth doing only after
/// the ordinary attempt returns `ENOSPC` rather than up front.
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
