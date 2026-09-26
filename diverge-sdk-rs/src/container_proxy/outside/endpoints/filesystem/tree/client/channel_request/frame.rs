//! What a client's channel request frame carries for a tree.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Stop watching.
///
/// The only thing the server asks of a running tree, and the whole of
/// what this channel carries. A payload is nothing at all.
///
/// # Why a tree has one and a read does not
///
/// Because a tree is a scope that does not end by itself. A read
/// answers and finishes; a tree reports changes for as long as the
/// scope stays open, so without this the only way to be done with one
/// is to stop reading — which tells the proxy nothing, and leaves it
/// watching a tree nobody is listening about.
///
/// # It has no answer, and does not need one
///
/// Nothing comes back on this channel. What comes back is the end of
/// the SCOPE — a
/// [`ResponseFinish`](crate::wire::frame::server::ServerFrame::ResponseFinish),
/// which already means nothing bearing this scope follows on any
/// channel. Finishing this one first would be a smaller way of saying
/// the same thing, moments earlier.
///
/// So the server sees its tree end the way a tree always ends: the
/// stream stops, with a finish rather than a silence.
///
/// # What it adds over closing the connection
///
/// Closing the connection ends the container. This ends one
/// subscription and nothing else: the other trees, the mounts, the
/// begin scope and everything on it go on.
///
/// # What it does not do
///
/// Touch the tree. A watch is a way of looking at the container's
/// filesystem, and looking away leaves everything where it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// Nothing at all. The frame is the whole message: which scope it
/// arrived on says which tree, and there is nothing else the server
/// can say to one.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): no bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, _out: &mut Writer<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// [`Infallible`](std::convert::Infallible): there is nothing to
    /// read.
    type Error = std::convert::Infallible;

    /// Whatever bytes arrive are ignored. There are none to send, so a
    /// peer that sent some knows something this version does not, and
    /// leaving room for it is cheaper than refusing it.
    fn decode(_bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Frame)
    }
}
