//! What a client's channel request frame carries for a watch.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Stop watching.
///
/// The only thing a caller asks of a running watch, and the whole of
/// what this channel carries. A payload is one byte and nothing else.
///
/// # Why a watch has one and the other volume scopes do not
///
/// Because a watch is the only one that does not end by itself. A
/// listing answers and finishes; a creation happens and finishes. A
/// watch reports changes for as long as the scope stays open, so
/// without this the only way to be done with one is to stop reading —
/// which tells the provider nothing, and leaves it watching a tree
/// nobody is listening about.
///
/// # It has no answer, and does not need one
///
/// Nothing comes back on this channel. What comes back is the end of
/// the SCOPE — a
/// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
/// which already means nothing bearing this scope follows on any
/// channel. Finishing this one first would be a smaller way of saying
/// the same thing, moments earlier.
///
/// So a caller sees its watch end the way a watch always ends: the
/// stream stops, with a finish rather than a silence. Which is the
/// distinction worth preserving — a watch that ended is not a watch
/// whose connection went.
///
/// # What it adds over closing the connection
///
/// Leaving ends this scope either way. The difference is that a
/// provider cannot tell a deliberate exit from a network that stopped
/// answering, and has to wait to find out — during which it is still
/// watching, still walking a tree, and still sending frames to a
/// receiver nobody holds. This is unambiguous and immediate.
///
/// # What it does not do
///
/// Touch the volume. A watch is a way of looking at one, and looking
/// away leaves everything where it was — the volume, its contents, and
/// any other caller watching the same tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// Nothing at all. The frame is the whole message: which channel it
/// arrived on says it is this scope's, and there is nothing else this
/// scope's caller can say.
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
