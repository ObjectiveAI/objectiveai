//! One client frame of a relayed daemon connection, held.

use bytes::Bytes;

use crate::wire::frame::FrameError;
use crate::wire::frame::client::ClientFrame;

/// One client frame the connector sent, of any type the wire has — its
/// token, in the handshake, among them — held whole: its bytes as the
/// wire lays them out, header and payload, so that a relay forwards
/// them as they are.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Owned(Bytes);

impl Owned {
    /// The frame `bytes` are, copied, if they decode as a client frame.
    pub fn read(bytes: &[u8]) -> Result<Owned, FrameError> {
        ClientFrame::decode(bytes)?;
        Ok(Owned(Bytes::copy_from_slice(bytes)))
    }

    /// The frame's bytes, for a wire.
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }

    /// The frame, decoded.
    pub fn frame(&self) -> Result<ClientFrame<'_>, FrameError> {
        ClientFrame::decode(&self.0)
    }
}
