//! What a server's channel request frame carries on an accept scope.

use crate::shared::daemons::Connection;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A connector arrived: the provider's half of its connection.
///
/// What arrives on the channel is the daemon's answer — its server
/// frames, one per channel response — and the daemon finishing the
/// channel is the daemon hanging up; a finish with nothing before it
/// is the daemon declining. The daemon's own half is a channel it
/// opens, quoting the id this carries.
///
/// # A struct, and no tag
///
/// One thing to ask is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate — and a tag byte is that
/// discriminant written on the wire, so it goes for the same reason.
/// The payload is the [`Connection`] as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame(
    /// Who, from where, under which id, and how they say they are let
    /// in.
    pub Connection,
);

impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, &self.0)
    }
}

impl Decode<'_> for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes).map(Frame)
    }
}
