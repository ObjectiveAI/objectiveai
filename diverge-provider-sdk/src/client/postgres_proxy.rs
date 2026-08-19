//! Splicing a plugin's database connection onto a real one.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

/// What connects a plugin to the caller's database.
///
/// A provider opens a Postgres channel because something inside a
/// container dialled the conduit it was given. The database lives with
/// the caller, so the bytes come out and this is what splices the far
/// end onto the real thing.
///
/// A plugin that never connects means the channel never exists, which is
/// what makes an opted-out plugin cost nothing rather than cost an idle
/// tunnel.
///
/// # It is the one that takes two channels
///
/// The other proxies answer something: a request arrives, an answer goes
/// back, the channel ends. A Postgres session is a long-lived
/// conversation with no natural top-level unit, so it does not fit that
/// and is not made to. One connection is TWO channels, one per
/// direction.
///
/// The provider opens the first, asking the caller to dial its database
/// and stream back what it says. The caller opens the second, quoting
/// the same
/// [`connection_id`](crate::endpoints::mcp_plugin::run::server::channel_request::Postgres::connection_id),
/// asking for what the plugin writes. This is called once per
/// connection, after both exist.
///
/// It is that way because only a responder can finish a channel, and a
/// connection has to be endable from both ends. One duplex channel could
/// say neither "the plugin is gone" nor "the database is gone"; two say
/// both, with nothing added to the protocol. It is the same inversion a
/// [`write`](crate::shared::container::write_path) makes, at the same
/// price of one round trip before the first byte.
///
/// # Several at once, and none of them related
///
/// A plugin holds a connection POOL, so this is called concurrently, on
/// the same `&self`, once per connection — which is what the [`Send`] and
/// [`Sync`] bounds above are for. An implementation must dial per call
/// rather than hand back something it is holding; one reusable
/// connection shared between calls would interleave two sessions onto
/// one socket, and pgwire has no way to tell them apart.
///
/// Nothing is shared between connections and nothing bounds how many run
/// at once. Their frames interleave freely on the one socket, which is
/// what keeps a large result set on one connection from blocking its
/// siblings.
///
/// # Never parsed
///
/// Which is what lets TLS negotiation and every protocol extension cross
/// untouched. A conduit that understood pgwire would have to keep up
/// with pgwire; one that does not is finished being written.
///
/// # Failure is a message, not a [`Result`]
///
/// There is no error variant on this channel and this returns no
/// [`Option`], because pgwire already says how a connection goes wrong.
/// A proxy that cannot reach the database sends an `ErrorResponse`
/// (`'E'`) as its first item and ends the stream, which is precisely
/// what the plugin's driver would see from a real server refusing it.
/// Ending the stream without saying anything is the other honest answer,
/// and is what a socket that dropped looks like.
///
/// An [`Option`] would be a third thing to check that collapses to one
/// of those two on the wire — both arms produce the same frames — so it
/// would be a distinction with no consequence.
///
/// Which is what each of these traits does, into a different vocabulary
/// each time — [`OciProxy`](super::oci_proxy::OciProxy) answers in HTTP
/// statuses, [`CommandProxy`](super::command_proxy::CommandProxy) in the
/// CLI's own, and this one in pgwire.
pub trait PostgresProxy: Send + Sync {
    /// Open one connection, and splice it onto the pair of channels
    /// carrying it.
    ///
    /// Everything the plugin writes arrives on `requests`; everything
    /// the database says goes back on the returned stream. The
    /// provider's channel finishes when that stream ends.
    ///
    /// # One call is one connection, and that is the whole of it
    ///
    /// Which is why nothing here names the connection. The protocol
    /// has an id for it — the provider mints one and both channels of
    /// the pair quote it — but it exists to pair two channels, and by
    /// the time this is called they are paired. What arrives is one
    /// connection's two ends, with nothing to match them against.
    ///
    /// Passing it anyway would put a number in the signature that no
    /// implementation has to read, out of a namespace this side does
    /// not mint in. A dispatcher that wants to correlate a log line
    /// across the socket is better placed to write it than this is: it
    /// holds the scope and both channel numbers as well.
    ///
    /// # What is on `requests`
    ///
    /// Payload bytes, with the frame header off them — what the plugin
    /// wrote and nothing else. Handing them on unmodified is the whole
    /// job; anything prepended is nine bytes of garbage in front of a
    /// startup message.
    ///
    /// They arrive as the RESPONSES on the channel the caller opened for
    /// them, which is why they can end. Calling them requests is a claim
    /// about pgwire — these are what the plugin asks the database — and
    /// not about which frame carried them.
    ///
    /// They are not messages, which is the other thing the name is at
    /// risk of suggesting. A pgwire message may span several items and
    /// several may share one, exactly as they would arriving off a
    /// socket. A driver on the far end is already prepared for that;
    /// nothing here needs to be.
    ///
    /// # Write them in the order they arrive, and do not wait
    ///
    /// Within one connection. Nothing is ordered against another
    /// connection's traffic, and nothing needs to be.
    ///
    /// Ordering is not a nicety here, it is the whole correlation
    /// mechanism. Postgres pipelines: a client may send a second query
    /// before the first has answered, and the extended query protocol
    /// exists to encourage exactly that. Nothing in a pgwire message
    /// identifies which request it answers — the server processes in
    /// order and replies in order, and a driver matches answers to
    /// questions BY POSITION. Two writes that swapped would not fail;
    /// they would succeed against the wrong statements.
    ///
    /// So there is no policy here of waiting for a reply before passing
    /// the next write on, and there could not be. A message larger than
    /// one frame spans several, so a proxy that paused after one to see
    /// what came back would be waiting on a reply to half a message,
    /// which is never coming. That is a deadlock, not a slowdown.
    ///
    /// The queue preserves order on the way in, and this is meant to
    /// drain it in order — writing each item onto the socket as it comes
    /// and reading the answers independently. What it must not do is
    /// take items off and put each on its own task; that is the right
    /// shape for a request/answer channel, and it is the wrong one for a
    /// socket.
    ///
    /// # Why it takes the receiver rather than returning a sink
    ///
    /// Because opening a connection is a network dial and the frames
    /// arrive on a queue that cannot wait for one. Taking the receiving
    /// end lets a dispatcher make the queue before it awaits anything,
    /// hand it over, and push the plugin's second write into it while
    /// this is still connecting — with the ordering the queue gives for
    /// free. A sink coming back would have nowhere to put the first
    /// write until this resolved, so it would have to grow this same
    /// queue in front of itself.
    ///
    /// The nearest alternative is taking a sender as a second argument
    /// and returning nothing, and it is a real one: no boxed stream, no
    /// [`Sync`] to satisfy. It loses on shape. Every proxy here returns
    /// what goes back, and that signature's future would instead be the
    /// connection's entire lifetime — a task rather than an answer,
    /// which a dispatcher would have to hold and account for
    /// differently from the other two.
    ///
    /// # Why one method and not two
    ///
    /// The exchange is two channels, so a trait with a method per
    /// channel is the obvious shape and is worse. The two directions
    /// share the one thing that cannot be handed over twice — the
    /// socket — so an implementation split across two calls would have
    /// to hold the dialled connection between them, keyed by whatever
    /// named it, in a map every implementation would write identically
    /// and the dispatcher already keeps.
    ///
    /// One call hands over both ends at once and lets the connection
    /// live on the implementation's own stack. What it costs is a
    /// [`tokio::spawn`] for the write pump, since the
    /// returned stream and the draining of `requests` have to run at
    /// the same time. That is the whole burden, and it is smaller than
    /// the bookkeeping the split would have required.
    ///
    /// # The channel is opened before this is called, deliberately
    ///
    /// So `requests` exists before anything is dialled, which looks
    /// like a caller committing to a connection it has not got yet. It
    /// is the right way round.
    ///
    /// The plugin wrote its startup message the instant it connected,
    /// and the provider has been holding it since. Waiting for the dial
    /// to succeed before asking for those bytes would put the round
    /// trip and the dial end to end instead of overlapping them, on
    /// every connection a pool opens.
    ///
    /// And declining costs nothing. A proxy that cannot dial returns a
    /// stream that ends; the caller finishes the provider's channel;
    /// the provider closes the plugin's socket and finishes this one.
    /// The exchange winds itself up, and what it cost was one channel
    /// that carried a few bytes nobody read.
    ///
    /// # `None` means the plugin hung up
    ///
    /// It is a real signal and the reason the exchange is shaped the way
    /// it is. `None` is the provider finishing the channel the caller
    /// opened, which says the plugin's socket ended and no further byte
    /// will ever be written by this connection.
    ///
    /// Not "nothing right now" — a quiet channel is a channel still
    /// running, and nothing here times one out. The right response is to
    /// hang up on the database and let the returned stream end.
    ///
    /// It matters most for the case the bytes cannot cover. A plugin
    /// that exits cleanly sends pgwire's `Terminate` (`'X'`) and the
    /// database closes on its own; a plugin that CRASHES sends nothing,
    /// and without this frame a caller would hold a backend for a client
    /// that no longer exists.
    ///
    /// The two directions are mirrors. This ending says the plugin is
    /// gone; the returned stream ending says the database is, and
    /// becomes a finish the provider acts on by shutting the plugin's
    /// socket.
    ///
    /// # Dropping the receiver is a real answer
    ///
    /// A queue with no reader grows, and nothing in this crate bounds
    /// it, so a proxy that has stopped caring should drop the receiver
    /// rather than hold it. That makes the dispatcher's writes fail,
    /// which is how it learns to finish the channel.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serves several plugins and a plugin may open
    /// several connections, so these overlap by design. It is spelled
    /// out rather than left to `async fn`, which promises nothing about
    /// the future it returns.
    ///
    /// # The bounds on the stream
    ///
    /// [`Send`] and `'static` because it is polled from wherever the
    /// answer is written, which is not where it was built. [`Sync`]
    /// because it is held behind a shared reference while that happens —
    /// the strictest of the three, and the one most likely to bite,
    /// since a stream needs only `&mut` to be polled.
    ///
    /// Written out rather than aliased, as in the two sibling proxies
    /// that return one. An alias would hide exactly those bounds from
    /// the signature a reader is checking theirs against.
    fn handle(
        &self,
        requests: UnboundedReceiver<Bytes>,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + Sync + 'static>>,
    > + Send;
}
