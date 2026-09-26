//! The file is at the destination.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The file landed in the other container.
///
/// It carries nothing, because saying so IS the whole message — no
/// fields, and no bytes on the wire either. The endpoints that allow
/// a transfer to fail wrap this in an enum of their own, as every
/// [`containers`](crate::endpoints::containers) scope does, and that
/// enum's tag does the discriminating.
///
/// # What a partial transfer leaves behind
///
/// Nothing at the destination. The write into the other container is
/// the proxy's own write, and holds what a
/// [`write_path`](crate::shared::containers::write_path::response::Frame)
/// holds: the destination is the old file, then nothing, then the new
/// one — never a prefix of the new one. A read that ends short, or in
/// an error, is a write abandoned, and the destination is as it was.
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
