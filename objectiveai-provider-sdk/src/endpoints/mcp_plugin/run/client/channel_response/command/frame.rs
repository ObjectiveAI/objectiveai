//! What a client's response frame carries on a command channel.

use std::convert::Infallible;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The payload of a
/// [`ClientFrame::ChannelResponse`](crate::frame::client::ClientFrame::ChannelResponse)
/// on a channel opened by
/// [`channel_request::Frame::Command`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame::Command).
///
/// One item the command produced, going back to the plugin that asked.
///
/// # One frame per item
///
/// Not one frame per command. A command that yields a thousand rows
/// sends a thousand of these and then finishes the channel, so the
/// plugin sees each as it lands rather than waiting for a document
/// assembled from all of them.
///
/// Which makes the channel's own finish the end of the command, and
/// there is no terminator in the payload because a second signal for
/// one fact is a second thing to disagree about. A command that fails
/// partway ends the same way any other does — the frames that arrived
/// are the items it produced, and the finish says there are no more.
///
/// # Opaque
///
/// The item's shape belongs to the command that produced it, and the
/// command vocabulary is the CLI's rather than this specification's.
/// Naming the shapes here would mean revising the protocol every time
/// a subcommand's output gained a field.
///
/// So a provider relays and never reads, in this direction as in the
/// other.
///
/// # Why a struct, where the outbound side has an enum
///
/// Because there is nothing to choose between. A provider's
/// [`channel_request::Frame`](crate::endpoints::mcp_plugin::run::server::channel_request::Frame)
/// has three variants because it opens three kinds of channel and a
/// reader must tell them apart. Once THIS channel is open its kind is
/// settled, and every frame on it is another item. An enum would imply
/// a decision nobody makes, and a tag byte would be a byte the far end
/// strips off every item it receives.
///
/// The same reason
/// [`postgres::Frame`](crate::endpoints::mcp_plugin::run::client::channel_response::postgres::Frame)
/// is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame<'a>(
    /// The item, borrowed from the frame it arrived in.
    pub &'a [u8],
);

/// Straight through. There is no encoding step because there is
/// nothing encoded — an item arrives as bytes and leaves as the same
/// bytes.
impl Encode for Frame<'_> {
    /// [`Infallible`]: copying a slice into a buffer has no failure
    /// mode, and saying so is better than inventing an error nobody
    /// can produce and every caller has to handle.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.0);
        Ok(())
    }
}

/// The bytes, kept. Decoding them would mean knowing which command
/// produced them, which is the one thing a relay does not.
impl<'a> Decode<'a> for Frame<'a> {
    /// [`Infallible`]: there is nothing to get wrong about a slice
    /// that is already the answer.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Frame(bytes))
    }
}
