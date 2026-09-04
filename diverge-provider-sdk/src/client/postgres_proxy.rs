//! Splicing a container's database connection onto a real one.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

/// What connects a container to the caller's database.
///
/// A provider opens a Postgres channel because something inside a
/// container dialled the conduit it was given — a plugin its declared
/// port, an agent its loop's port `14980`. The database lives with the
/// caller, so the bytes come out and this is what splices the far end
/// onto the real thing.
///
/// A container that asked for no database, or that never connects,
/// means the channel never exists — which is what makes an opted-out
/// plugin, or an agent whose state is not rows, cost nothing rather
/// than cost an idle tunnel.
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
/// the same connection id
/// ([the plugin's](crate::endpoints::mcp_plugin::run::server::channel_request::Postgres::connection_id),
/// [the loop's](crate::endpoints::agentic_loop::run::server::channel_request::Postgres::connection_id)),
/// asking for what the container writes. This is called once per
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
/// A database client holds a connection POOL, so this is called
/// concurrently, on the same `&self`, once per connection — which is
/// what the [`Send`] and [`Sync`] bounds above are for. An implementation must dial per call
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
/// what the container's driver would see from a real server refusing
/// it.
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
///
/// # It is generic in the request, alone among these
///
/// The other proxies take a [`shared`](crate::shared) type or bytes,
/// and nothing else in [`client`](crate::client) names anything from
/// [`endpoints`](crate::endpoints). This one is handed THE REQUEST
/// that started the run — an MCP plugin's
/// [`request::Frame`](crate::endpoints::mcp_plugin::run::client::request::Frame)
/// or an agentic loop's
/// [`request::Frame`](crate::endpoints::agentic_loop::run::client::request::Frame)
/// — and that is a deliberate exception rather than an oversight.
///
/// A caller decides what a container's connection may REACH, and it
/// decides that from who is asking and what is running. Neither is
/// expressible in a shared type, because both are facts about the
/// endpoint's request. A trait that could not say what a security
/// decision rests on would be the worse violation of the two.
///
/// Two endpoints carry the exchange, so the trait is generic in the
/// request rather than naming one: a caller serving both implements
/// it twice, once per request type, and a caller serving one
/// implements it once. Nothing else about the exchange differs
/// between them.
pub trait PostgresProxy<Request>: Send + Sync {
    /// Open one connection, and splice it onto the pair of channels
    /// carrying it.
    ///
    /// Everything the container writes arrives on `requests`;
    /// everything the database says goes back on the returned stream.
    /// The provider's channel finishes when that stream ends.
    ///
    /// # What `request` is for
    ///
    /// Deciding what this connection may reach.
    ///
    /// A caller that puts its containers in compartments — so that
    /// one cannot read what another wrote — has to choose the
    /// compartment from something, and the request is where that
    /// something is. For a plugin,
    /// [`identity`](crate::endpoints::mcp_plugin::run::client::request::Frame::identity)
    /// says on whose behalf it runs and
    /// [`image`](crate::endpoints::mcp_plugin::run::client::request::Frame::image)
    /// says what is running, and neither alone is enough: the same
    /// image on behalf of two agents is two compartments, and two
    /// images on behalf of one agent are also two. For a loop, the
    /// [`agent`](crate::endpoints::agentic_loop::run::client::request::Frame::agent)
    /// is what is running, and whose behalf is the caller's own
    /// knowledge — it sent the request.
    ///
    /// Nothing in this specification performs that separation or
    /// requires it. What this argument does is make it POSSIBLE, which
    /// it was not when all a proxy received was a stream of bytes —
    /// every plugin's connection looked alike, so every plugin's
    /// connection had to be trusted alike.
    ///
    /// # Why the whole frame, and not the two fields
    ///
    /// Because the policy is the caller's and this specification
    /// should not be the thing that bounds it. Narrowing to two
    /// fields would be a guess about what a compartment is derived
    /// from, and a signature change the first time it is derived from
    /// something else — a plugin's
    /// [`arguments`](crate::endpoints::mcp_plugin::run::client::request::Frame::arguments)
    /// being the obvious next one, since its own configuration may
    /// name what it expects to reach.
    ///
    /// It costs nothing to hand over. This is the caller's own request
    /// coming back to it, already decoded, already held for the run's
    /// life.
    ///
    /// # It is the same every call
    ///
    /// One run is one scope is one request, and every connection
    /// opened under it gets that request. So a proxy deriving a
    /// compartment from it derives the same compartment every time,
    /// which is the point — a run's own connections belong together,
    /// and it is other runs they are being kept apart from.
    ///
    /// A borrow, because it belongs to the run rather than to any one
    /// connection and outlives all of them. An implementation that
    /// wants to keep something out of it clones what it wants.
    ///
    /// # One call is still one connection
    ///
    /// Nothing here names the connection, because nothing needs to.
    /// The protocol has an id for it — the provider mints one and both
    /// channels of the pair quote it — but it exists to pair two
    /// channels, and by the time this is called they are paired. What
    /// arrives is one connection's two ends, with nothing to match
    /// them against.
    ///
    /// # What is on `requests`
    ///
    /// Payload bytes, with the frame header off them — what the
    /// container wrote and nothing else. Handing them on unmodified is
    /// the whole job; anything prepended is nine bytes of garbage in
    /// front of a startup message.
    ///
    /// They arrive as the RESPONSES on the channel the caller opened for
    /// them, which is why they can end. Calling them requests is a claim
    /// about pgwire — these are what the container asks the database —
    /// and not about which frame carried them.
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
    /// and returning nothing, and it is a real one: no boxed stream at
    /// all. It loses on shape. Every proxy here returns
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
    /// The container wrote its startup message the instant it
    /// connected, and the provider has been holding it since. Waiting for the dial
    /// to succeed before asking for those bytes would put the round
    /// trip and the dial end to end instead of overlapping them, on
    /// every connection a pool opens.
    ///
    /// And declining costs nothing. A proxy that cannot dial returns a
    /// stream that ends; the caller finishes the provider's channel;
    /// the provider closes the container's socket and finishes this
    /// one.
    /// The exchange winds itself up, and what it cost was one channel
    /// that carried a few bytes nobody read.
    ///
    /// # `None` means the container hung up
    ///
    /// It is a real signal and the reason the exchange is shaped the way
    /// it is. `None` is the provider finishing the channel the caller
    /// opened, which says the container's socket ended and no further
    /// byte will ever be written by this connection.
    ///
    /// Not "nothing right now" — a quiet channel is a channel still
    /// running, and nothing here times one out. The right response is to
    /// hang up on the database and let the returned stream end.
    ///
    /// It matters most for the case the bytes cannot cover. A process
    /// that exits cleanly sends pgwire's `Terminate` (`'X'`) and the
    /// database closes on its own; one that CRASHES sends nothing, and
    /// without this frame a caller would hold a backend for a client
    /// that no longer exists.
    ///
    /// The two directions are mirrors. This ending says the container
    /// is gone; the returned stream ending says the database is, and
    /// becomes a finish the provider acts on by shutting the
    /// container's socket.
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
    /// Because a provider serves several runs and a run may open
    /// several connections, so these overlap by design. It is spelled
    /// out rather than left to `async fn`, which promises nothing about
    /// the future it returns.
    ///
    /// # The future is not `'static`
    ///
    /// It borrows `request` for as long as it runs, so a dispatcher
    /// putting each connection on its own task holds the run's request
    /// behind something shared and borrows it inside the task, rather
    /// than trying to send a reference into one.
    ///
    /// The dial is the only thing that happens before the returned
    /// stream exists, so the borrow is brief. What runs for the
    /// connection's life is the STREAM, and that is `'static`.
    ///
    /// # The bounds on the stream
    ///
    /// [`Send`] and `'static` because it is polled from wherever the
    /// answer is written, which is not where it was built.
    ///
    /// [`Sync`] is NOT required, and used to be. Whoever writes the
    /// answer owns this and polls it through `&mut`, so a shared
    /// reference to it never exists — and requiring one turned away the
    /// obvious way to write a stream, since an `async_stream` generator
    /// is [`Sync`] only if everything it awaits is.
    ///
    /// Written out rather than aliased, as in the sibling proxies that
    /// return one. An alias would hide exactly those bounds from the
    /// signature a reader is checking theirs against.
    fn handle(
        &self,
        request: &Request,
        request_receiver: UnboundedReceiver<Bytes>,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>,
    > + Send;
}
