//! Being attached to a laboratory, and what a connector can say to it.

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::client::handle::Handle;

/// An attachment to somebody else's laboratory.
///
/// Half of what [`execute`](super::execute) gives back. The other half
/// is an [`ExecuteStream`](super::ExecuteStream) of the container's
/// filesystem, and the two do opposite jobs: that is everything the
/// provider says without being asked, and this is how a connector asks
/// for anything.
///
/// # Holding it is what keeps the connection open
///
/// Dropping it leaves the laboratory — see the [`Drop`] impl. That
/// makes dropping the ordinary way to be done rather than a way to
/// abandon one, and it is why the two halves are worth telling apart:
/// letting the filetree go costs a connector its view of the
/// filesystem, and letting this go costs it the connection.
///
/// It takes nothing with it. The container goes on running, other
/// connectors stay attached, and the runner sees one fewer connection
/// — stopping a laboratory belongs to whoever created it and is
/// [`Stop`](crate::endpoints::laboratories::run::client::channel_request::Frame::Stop)
/// on their scope, not anything a connector can reach.
///
/// # There is nothing on it yet
///
/// A connector has four things it can ask for —
/// [`Mcp`](super::channel_request::Frame::Mcp),
/// [`Read`](super::channel_request::Frame::Read),
/// [`Write`](super::channel_request::Frame::Write) and
/// [`Transfer`](super::channel_request::Frame::Transfer) — and none of
/// them is written. This is what they will hang off, and it already
/// carries what they need.
///
/// Three of the four need only the write half of the connection and
/// the scope number, both of which are here:
/// [`send_channel_request`](Handle::send_channel_request) hands back a
/// channel with its own receiver, so an exchange is opened, read and
/// finished without anything else being kept.
///
/// A write is the exception. Its content does not travel on the
/// channel that asked for it — it goes back on a channel the PROVIDER
/// opens, quoting a
/// [`write_id`](crate::shared::container::write_path::request::Request::write_id),
/// because only a responder can finish a channel. So several writes can
/// be outstanding at once with their content requests arriving
/// together, and telling them apart is a task and a registry rather
/// than one method's business.
///
/// This already holds the receiver those requests arrive on. Deciding
/// what reads it is the first part of writing a write.
#[must_use = "dropping the handle leaves the laboratory"]
#[derive(Debug)]
pub struct ExecuteHandle {
    /// What everything a connector says goes out over.
    ///
    /// A [`Handle`] rather than pre-encoded frames, and for the
    /// disconnect in particular the difference matters: a frame carries
    /// a scope number it cannot re-check, and by the time a destructor
    /// runs that number may belong to somebody else.
    /// [`send_channel_request`](Handle::send_channel_request) takes
    /// back what the router has closed and then looks the scope up, so
    /// a scope that is gone sends nothing.
    handle: Handle,
    /// The scope this connection is running in, as this end numbered
    /// it.
    ///
    /// Every channel a connector opens carries it, and so does the
    /// disconnect. The router already sorted the incoming frames by it,
    /// so nothing here reads it for that.
    scope: u32,
    /// The disconnect, encoded and ready.
    ///
    /// Built once by [`execute`](super::execute) through
    /// [`channel_request::Frame`](super::channel_request::Frame) rather
    /// than written as the byte it happens to be. A destructor is a
    /// poor place to be encoding anything, and what a disconnect looks
    /// like on the wire is not this type's to know.
    disconnect_request: Bytes,
    /// The channels the provider opens inside this scope.
    ///
    /// Nothing reads it yet, and nothing can arrive on it yet either.
    /// A provider opens exactly one kind of channel on a connector —
    /// asking for the content of a file the connector said it wanted to
    /// write — and there is no way to ask for a write until
    /// [`Write`](super::channel_request::Frame::Write) has a method.
    ///
    /// It is kept rather than dropped, which is the opposite of what a
    /// [`watch`](crate::endpoints::volumes::watch) does with the same
    /// receiver. There, a channel request would be a stray from a
    /// provider that should not have opened one, so dropping it
    /// dead-letters the mistake. Here one is expected by design, and
    /// whatever ends up serving writes will read this.
    ///
    /// # It will not be enough on its own
    ///
    /// Several writes can be outstanding at once and their content
    /// requests all arrive here, told apart only by the
    /// [`write_id`](crate::shared::container::write_bytes::request::Request::write_id)
    /// inside them. So a `write` method cannot simply read this:
    /// something has to take frames off it and route each to whoever is
    /// supplying that write's bytes, which is a task and a registry of
    /// outstanding writes rather than one method's business.
    ///
    /// That registry is the first thing to decide when writes are
    /// written. The receiver is here because whatever owns it will own
    /// this.
    #[allow(
        dead_code,
        reason = "held for the write content requests nothing serves yet"
    )]
    request_receiver: UnboundedReceiver<Bytes>,
}

impl ExecuteHandle {
    /// Take the pieces, from the [`execute`](super::execute) that has
    /// them.
    ///
    /// Not public. A connection exists because a request went out, so
    /// the only thing that can honestly make one of these is the thing
    /// that sent it.
    pub(super) fn new(
        handle: Handle,
        scope: u32,
        disconnect_request: Bytes,
        request_receiver: UnboundedReceiver<Bytes>,
    ) -> Self {
        ExecuteHandle {
            handle,
            scope,
            disconnect_request,
            request_receiver,
        }
    }
}

/// Leave the laboratory.
///
/// # It spawns rather than sends
///
/// A destructor cannot await, and writing a frame means locking a
/// connection and waiting on a socket. What it can do is hand the whole
/// thing to a runtime and return, which is all this does.
///
/// [`try_current`](tokio::runtime::Handle::try_current) rather than
/// [`tokio::spawn`], because `spawn` PANICS outside a runtime and a
/// destructor is the worst place in a program to do that. No runtime
/// means no disconnect, which leaves things exactly as they were before
/// this existed.
///
/// # There is no terminal state to check, and none is needed
///
/// A [`watch`](crate::endpoints::volumes::watch)'s stream checks one
/// before sending, because it holds the receiver that would know. This
/// does not — the
/// [`ExecuteStream`](super::ExecuteStream) has it, and the two halves
/// are separable on purpose, so a connector that dropped the stream
/// long ago would leave this with nothing to consult.
///
/// The guard that matters was never that check anyway. It is one level
/// down: [`send_channel_request`](Handle::send_channel_request) takes
/// back what the router has closed and then looks the scope up, so a
/// scope that has ended sends nothing — and cannot disconnect somebody
/// who was handed the same number afterwards. The field check is an
/// early-out; this is the close.
///
/// # The answer is dropped
///
/// Nothing answers a disconnect; what answers it is the scope's own
/// finish. So the channel this opens is abandoned immediately, and its
/// entry in the router lingers until the scope closes — which is the
/// thing the disconnect is provoking.
///
/// # What it adds over just going away
///
/// Leaving ends the scope either way. The difference is that a provider
/// cannot tell a deliberate exit from a network that stopped answering,
/// and has to wait to find out — during which the runner has not been
/// told this connector is gone. This is unambiguous and immediate.
impl Drop for ExecuteHandle {
    fn drop(&mut self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let handle = self.handle.clone();
        let scope = self.scope;
        let disconnect_request = self.disconnect_request.clone();
        runtime.spawn(async move {
            let _ =
                handle.send_channel_request(scope, &disconnect_request).await;
        });
    }
}
