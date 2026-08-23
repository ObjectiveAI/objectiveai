//! A container that is running, and what can be done with one.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Sink, Stream};

use crate::shared::error::Error;
use crate::shared::http::{request, response};

/// A running container, as far as this crate needs one.
///
/// What a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// hands back. The type itself is the provider's — a process handle, a
/// name, an instance id, a struct with all three — and this is the part
/// of it this crate has to be able to reach.
///
/// # What belongs here
///
/// Whatever needs something only the thing that deployed the container
/// has. Not "what a provider cannot do from outside", which was the
/// first answer and is too narrow — plenty of what a provider does to a
/// container is done from outside it, and still needs a fact the deploy
/// was the only thing to learn.
///
/// So these are facts a provider keeps to itself. Where the container's
/// filesystem is, for [`read`](Self::read) and [`write`](Self::write).
/// How to end it, for [`stop`](Self::stop). How to reach a port inside
/// it, for [`http_serve`](Self::http_serve) — a container's port has a
/// host-side address only on some runtimes, and on the rest the way in
/// is the provider's own control plane, so it is a method and never an
/// address.
///
/// A filetree will join them when it is written, since watching a
/// filesystem is knowing where one is. A transfer will not — it is a
/// read on one container and a write on another, both already here.
///
/// # Ports are three methods, not one
///
/// There was a `connect` that handed back a byte pipe, and everything
/// spoken to a container rode one. It is gone, because the things
/// riding it were not one thing.
///
/// [`http_call`](Self::http_call) is the provider asking and
/// [`http_serve`](Self::http_serve) is the provider answering: one
/// takes a request and returns an answer, the other returns requests
/// and takes answers. Between them they carry everything a container
/// says in HTTP, which is an MCP server, an agent's tool calls, and a
/// plugin's commands.
///
/// [`postgres_serve`](Self::postgres_serve) is the exception, and it is
/// the only one still shaped like a socket — because pgwire is a duplex
/// conversation rather than a series of exchanges, and there was
/// nothing in it to hand over instead.
///
/// Which is why the pipe was the wrong shape for all of them. It was
/// built for the one case that needs it, and the other three were made
/// to speak through a socket when what they had was requests.
///
/// # `Send` and `Sync`
///
/// Because a scope is served by more than one task — a dispatcher
/// reading channel requests, a pump answering one — and a container is
/// reached from any of them, behind a shared reference.
///
/// Which is also why every method takes `&self`. Nothing here consumes
/// a container, including [`stop`](Self::stop): it is stopped while
/// whatever is holding it still holds it, and dropping it afterwards is
/// a separate act that this crate does not define.
pub trait Container: Send + Sync {
    /// Why something a container was asked for did not happen.
    ///
    /// The provider's own, for the reason
    /// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
    /// is: a file that would not open, a runtime that would not stop, a
    /// disk that filled — these belong to a runtime and a kernel, and
    /// this crate names neither.
    ///
    /// One type for both methods that can fail, because a provider
    /// that told them apart would be doing it for its own benefit
    /// rather than this crate's. Nothing here branches on which
    /// operation failed; what it does with one is put it on the wire.
    ///
    /// [`stop`](Self::stop) is not one of them — see it for why.
    ///
    /// It need not be the same type a deploy fails with. A provider
    /// whose deploy and whose reads go wrong in the same ways uses one
    /// for both, and one whose reads can only fail in ways a deploy
    /// never could says so.
    ///
    /// [`Send`] and `'static` for the same reasons every other error
    /// here is: the future carrying it is [`Send`], so its output has
    /// to be, and it outlives the call that produced it.
    type Error: Send + 'static;


    /// What arrives when the container asks for something.
    ///
    /// One item per request, each with the [`HttpResponseWriter`] that
    /// answers that one. The stream ends when the container has no more
    /// to ask — see [`http_serve`](Self::http_serve).
    ///
    /// # The request is bytes, and they are a
    /// [`Request`](crate::shared::http::request::Request)
    ///
    /// Encoded as that type encodes: JSON, with the body nested
    /// verbatim as the [`RawValue`](serde_json::value::RawValue) it
    /// already is. An implementation produces one; a consumer decodes
    /// it.
    ///
    /// # Why not the type itself
    ///
    /// Because a [`Request`](crate::shared::http::request::Request)
    /// borrows its body from the buffer it was decoded out of, so it
    /// cannot be a stream item — an item has to stand on its own once
    /// yielded, and that one points into something.
    ///
    /// [`Bytes`] is that buffer, made ownable. A consumer holds the
    /// item and decodes a request that borrows from it, which is the
    /// same arrangement every frame in this crate already has: the
    /// bytes are the thing that lives, and the typed view is a way of
    /// reading them.
    ///
    /// It also means a relay does not have to re-encode. What a
    /// container asked is already in the form a channel request
    /// carries, so forwarding it is a copy at worst and a
    /// [`slice_ref`](Bytes::slice_ref) at best — where a decoded
    /// request would have been taken apart and put back together for
    /// nothing.
    type HttpRequestStream: Stream<Item = (Bytes, Self::HttpResponseWriter)>
        + Send
        + Unpin
        + 'static;

    /// Where one of those requests is answered.
    ///
    /// Its error is [`Self::Error`] because a failure to answer is a
    /// failure of the same connection the request arrived on, and
    /// splitting them would be inventing a distinction a provider does
    /// not have.
    type HttpResponseWriter: HttpResponseWriter<Error = Self::Error>
        + Send
        + 'static;

    /// The database connections a container opens, as it opens them.
    ///
    /// One item per connection, and nothing before it. A plugin holds a
    /// POOL, so this is the shape that says how many there are: however
    /// many turn up.
    type PostgresConnections: Stream<
            Item = (Self::PostgresReader, Self::PostgresWriter),
        > + Send
        + Unpin
        + 'static;

    /// What one of those connections says.
    ///
    /// Whatever comes off the socket, in whatever pieces it comes off
    /// in. The pieces mean nothing — pgwire has its own framing and it
    /// is not this one — which is fine, because nothing here reads
    /// them: each piece becomes one frame and goes to the caller as it
    /// arrives.
    type PostgresReader: Stream<Item = Result<Bytes, Self::Error>>
        + Send
        + Unpin
        + 'static;

    /// Where the answer to one of those connections goes.
    ///
    /// What the caller's database said, on its way back in. Also
    /// unframed, and for the same reason.
    type PostgresWriter: Sink<Bytes, Error = Self::Error>
        + Send
        + Unpin
        + 'static;

    /// Serve HTTP to something inside the container.
    ///
    /// The container is the CLIENT here. It makes requests and this end
    /// answers them, which is the direction an agent's tool calls
    /// travel: an
    /// [`agentic_loop`](crate::endpoints::agentic_loop::run) runs its
    /// agent beside the provider and the MCP servers live with the
    /// caller, so a tool call has to leave the container before it can
    /// go anywhere.
    ///
    /// # It is the provider that dials
    ///
    /// Even though the provider is the one serving. A provider cannot
    /// put a listening socket inside somebody else's container, so
    /// anything a container wants served FOR it is something the
    /// container has to be listening on — and the provider connects and
    /// then answers on a connection it opened.
    ///
    /// Which is why this takes a port and does not hand back an
    /// address. The port is the container's, declared in
    /// [`ports`](super::deployment::Deployment::ports), and how a
    /// provider reaches it is the provider's business.
    ///
    /// # Requests, not a byte pipe
    ///
    /// The implementation speaks HTTP and this crate does not. Which is
    /// the whole point of the shape: a pipe would have every consumer
    /// parsing request heads and decoding chunked bodies, and there is
    /// no version of that which is this protocol's business.
    ///
    /// What arrives is one request per item, already separated from the
    /// next and already stripped of the framing that separated them.
    /// That it arrives as bytes rather than as a struct is a different
    /// question, answered on
    /// [`HttpRequestStream`](Self::HttpRequestStream).
    ///
    /// It also means the framing question is answered by HTTP rather
    /// than by anything invented here. Several tool calls at once are
    /// several requests, told apart by the protocol that already tells
    /// requests apart, on however many connections the implementation
    /// finds convenient.
    ///
    /// # Answers can be given in any order
    ///
    /// Each request arrives with its own writer, and nothing pairs a
    /// writer with the one that came before it. A consumer that takes
    /// three requests and answers the third first has done nothing
    /// wrong, and an implementation has to be able to carry that —
    /// which for HTTP/1.1 means a connection each, and for anything
    /// newer means a stream each.
    ///
    /// # The stream ends when the container stops asking
    ///
    /// Cleanly, and it says nothing about the container. A plugin that
    /// has no more tool calls to make and a plugin that has finished
    /// its work look the same from here, because they are the same
    /// thing from here: the conversation is over and the container's
    /// own life is [`stop`](Self::stop)'s business.
    ///
    /// An [`Err`] is a failure to START serving — nothing listening on
    /// that port, or a port that was never declared. A failure after
    /// that ends the stream, since a request that cannot be received is
    /// indistinguishable from one that was never sent.
    fn http_serve(
        &self,
        port: u16,
    ) -> impl Future<Output = Result<Self::HttpRequestStream, Self::Error>> + Send;

    /// Make one HTTP request to something inside the container.
    ///
    /// The other direction from [`http_serve`](Self::http_serve), and
    /// the more ordinary one: a plugin's MCP server, a laboratory's, an
    /// agent image asked to start a run. The container is the server
    /// and this end is its client.
    ///
    /// # One call is one exchange
    ///
    /// No session, no connection to hold, nothing kept between calls.
    /// Which is what MCP over Streamable HTTP already is — a series of
    /// discrete requests over a session identified by a HEADER rather
    /// than by anything at the transport layer — so a connection held
    /// open between them would be a pool this crate was managing on
    /// somebody else's behalf.
    ///
    /// Whether an implementation actually opens a socket each time is
    /// its own business. Nothing here can tell, and nothing here should
    /// care.
    ///
    /// # The request is not encoded
    ///
    /// Unlike [`http_serve`](Self::http_serve), whose items are bytes.
    /// The asymmetry is not an inconsistency: there the bytes ARE the
    /// buffer a borrowed request needs, and here the request is an
    /// argument that lives for the call, so there is nothing to own it
    /// and no reason to serialize it first.
    ///
    /// # The answer is a head and a body
    ///
    /// Separately, and the body is one piece or a stream of them — see
    /// [`Body`]. Which is the shape MCP forces: a `Content-Type` of
    /// `application/json` introduces one document and the exchange is
    /// over, and `text/event-stream` introduces a stream held open for
    /// the session. Collapsing them would mean buffering the second
    /// into the first, which for a session stream means buffering until
    /// it closes and answering nothing until then.
    ///
    /// It is the same pair a
    /// [`McpProxy`](crate::client::mcp_proxy::McpProxy) hands back on
    /// the other half of this crate, for the same reason and with the
    /// same meaning.
    ///
    /// # What an [`Err`] is
    ///
    /// No answer at all — nothing listening on that port, a port never
    /// declared, a connection that broke before a head arrived. A
    /// response with a status in it is [`Ok`], including a `500`: what
    /// a status MEANS belongs to whatever asked, and this layer would
    /// be guessing.
    ///
    /// A failure after the head has gone is neither. There is nowhere
    /// to report one — the status is already sent and HTTP has no way
    /// to take it back — so the body simply stops, which is what an
    /// ordinary HTTP connection dropping looks like and is what this
    /// stands in for.
    fn http_call(
        &self,
        port: u16,
        request: request::Request<'_>,
    ) -> impl Future<Output = Result<(response::Head, Body), Self::Error>> + Send;

    /// Take the database connections a container opens.
    ///
    /// A plugin that was given a
    /// [`postgres_port`](crate::endpoints::mcp_plugin::run::client::request::Frame::postgres_port)
    /// has a database, and the database lives with the CALLER. So the
    /// plugin connects, and every connection it opens has to be carried
    /// out of the container and offered to whoever holds the data.
    ///
    /// # It is bytes, and it is the one thing that has to be
    ///
    /// Everything else spoken to a container is an HTTP exchange, which
    /// is what lets [`http_call`](Self::http_call) and
    /// [`http_serve`](Self::http_serve) hand over requests instead of a
    /// socket. This cannot be: pgwire is a duplex conversation with its
    /// own framing, its own pipelining, and messages that arrive
    /// unprompted, and there is no exchange in it to hand over.
    ///
    /// Parsing it would mean tracking it — a wire protocol that gains
    /// messages on somebody else's schedule, inside the crate that is
    /// this protocol's normative artifact. So the bytes go through
    /// unread, which is also what the caller's
    /// [`PostgresProxy`](crate::client::postgres_proxy::PostgresProxy)
    /// takes and gives back.
    ///
    /// # A stream, because the count is the plugin's
    ///
    /// A plugin holds a connection pool and opens as many as it turns
    /// out to want, so nothing here can say how many there will be or
    /// when. One item per connection is the whole answer: an
    /// implementation yields when the plugin opens one, and a consumer
    /// that is not ready simply has not polled yet.
    ///
    /// Which is what makes it possible to stop GUESSING. There is no
    /// asking for a connection that might be wanted and no reading a
    /// first byte to find out whether it was — the item's existence is
    /// the fact, and it arrives when the fact is true.
    ///
    /// # A half-close is not needed
    ///
    /// Unlike the byte pipe this replaces, which had a conduit
    /// depending on the read half outliving the write half. Postgres
    /// says goodbye with a MESSAGE — `Terminate` — and then closes, so
    /// nothing here needs the two halves to end separately and an
    /// implementation is free to treat either as the connection's end.
    ///
    /// # Ends and errors
    ///
    /// The stream ends when the plugin opens no more, which says
    /// nothing about the container: a plugin between queries and a
    /// plugin that has finished look identical from here, because they
    /// are identical from here.
    ///
    /// An [`Err`] is a failure to START — nothing listening on that
    /// port, or a port never declared. A failure after that ends the
    /// stream. A failure on one CONNECTION is that connection's, and
    /// arrives in its own reader.
    fn postgres_serve(
        &self,
        port: u16,
    ) -> impl Future<Output = Result<Self::PostgresConnections, Self::Error>> + Send;

    /// Stop it.
    ///
    /// Returns when the container is stopped, the way a deploy returns
    /// when it is running. Not when a stop has been requested, not when
    /// a signal has been sent — an implementation that waits for
    /// something waits before it resolves.
    ///
    /// # It cannot fail, and the reason is not optimism
    ///
    /// Two things would have gone in a [`Result`] and neither belongs
    /// there.
    ///
    /// A container that is ALREADY GONE is not a failure. It exited on
    /// its own, or crashed, or something else stopped it — and what was
    /// asked for is the state rather than the act, so a container that
    /// is not running is the whole of it. An error there would be one
    /// every caller has to recognise and then ignore, which is writing
    /// the same rule in every caller instead of once here.
    ///
    /// A container that WILL NOT stop is a failure, and there is
    /// nothing to do about it. No frame means "the container would not
    /// stop": a scope ends when its provider finishes it, and it
    /// finishes either way. The error would have nowhere to go and no
    /// caller able to act on it.
    ///
    /// What an implementation does about the second is its own business
    /// — retry it, log it, leave it for whatever reaps stragglers. That
    /// is operational, and this protocol has no opinion.
    ///
    /// Which leaves one thing this future means: as far as the provider
    /// is concerned, that container is done.
    ///
    /// # What it does to the scope is not this
    ///
    /// A scope ends when its provider finishes it, and stopping a
    /// container is one of the things that leads to that. This does not
    /// send a frame and does not know there is one to send.
    fn stop(&self) -> impl Future<Output = ()> + Send;


    /// Read one file out of it.
    ///
    /// The future resolves when the read has STARTED, not when it has
    /// finished — what it resolves to is the file, and that is where
    /// the reading shows up. A file that cannot be opened says so as
    /// the stream's first item rather than as a failure here, which is
    /// what lets both endings arrive by one route.
    ///
    /// # One file, never a directory
    ///
    /// See [`read`](crate::shared::container::read) for why. A path
    /// that names a directory is a read that fails, not a read that
    /// produces something else.
    ///
    /// # The pieces are the implementation's to choose
    ///
    /// One item or a thousand, and how large each is, carries no
    /// meaning — the far end concatenates. What it should not do is
    /// hold the whole file to send it as one piece: a caller reading a
    /// large file gets it as it comes, and buffering here would undo
    /// that for every reader.
    ///
    /// # An [`Err`] ends it
    ///
    /// Whatever arrived before it is a prefix of the file, and nothing
    /// says how much is missing. A reader cannot tell a refused read
    /// from a truncated one, which is what the endpoint's own frame
    /// says too: "the file was not read, or not all of it".
    ///
    /// # The path
    ///
    /// Components from the container's root, the same frame of
    /// reference a [`filetree`](crate::shared::filetree) stream uses.
    /// Components rather than a joined string, because joining invents
    /// a separator that then has to be escaped out of names containing
    /// it.
    fn read(
        &self,
        path: &[String],
    ) -> impl Future<
        Output = Pin<
            Box<dyn Stream<Item = Result<Bytes, Self::Error>> + Send>,
        >,
    > + Send;

    /// Write one file into it.
    ///
    /// Resolves when the file is at the path, or when it is not. Unlike
    /// [`read`](Self::read), this does not resolve early: there is one
    /// answer and it is not known until the content has been consumed.
    ///
    /// # A write replaces, and leaves nothing partial
    ///
    /// Whatever is at the path is replaced, whole. An implementation
    /// writes to a temporary and renames it into place, so the path
    /// holds the old file, then nothing, then the new one — never a
    /// prefix of the new one. That holds whether this returns [`Ok`] or
    /// not, and callers are told it does.
    ///
    /// # The content can come from two places
    ///
    /// Which is why its errors are a [`ContentError`] rather than one
    /// type. A caller's content arrives over the wire and fails in the
    /// caller's vocabulary; a
    /// [`transfer`](crate::shared::container::transfer)'s comes from a
    /// [`read`](Self::read) on another container in this same process
    /// and fails in the provider's. See [`ContentError`] for why
    /// neither collapses into the other.
    ///
    /// Either way an [`Err`] item is not a failure of the container
    /// being written into. It is the source saying it has no more to
    /// give — what the endpoint's frame calls "the full content was not
    /// streamed" — and an implementation that sees one abandons the
    /// write. There is nothing partial at the path either way.
    ///
    /// # What it RETURNS is still the provider's
    ///
    /// [`Self::Error`](Self::Error), whatever ended the content. A
    /// write abandoned because its source stopped is still a write that
    /// did not land, and this reports what the container made of that
    /// rather than repeating why the bytes ran out — which the thing
    /// supplying them already knows.
    ///
    /// # Boxed rather than generic
    ///
    /// The stream is built by whatever is relaying the caller's
    /// content, and there is one shape of it. A type parameter would
    /// let an implementation be generic over something that never
    /// varies, at the cost of a bound on every signature that mentions
    /// one.
    ///
    /// [`Send`] and `'static` because it is polled wherever the write
    /// happens. Not [`Sync`] — one task owns it and polls it through
    /// `&mut`, so a shared reference to it never exists.
    fn write(
        &self,
        path: &[String],
        content: Pin<
            Box<
                dyn Stream<Item = Result<Bytes, ContentError<Self::Error>>>
                    + Send,
            >,
        >,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// The body of an answer: all of it, or a piece at a time.
///
/// What [`Container::http_call`] hands back beside the head.
///
/// # Why the shape is a choice at all
///
/// Because MCP over Streamable HTTP answers in two ways, and the head
/// says which. A `Content-Type` of `application/json` introduces one
/// document, complete, and a request that got one is over. A
/// `text/event-stream` introduces an event stream held open for the
/// session, which is where a server pushes notifications and the
/// answers to things it was asked while it was thinking.
///
/// Something that had to pick one would have to buffer the second into
/// the first — which for a stream held open for a session means
/// buffering forever, and answering nothing until it closed.
///
/// # Both end the same way
///
/// The answer is over when the body is: after the one piece, or after
/// the stream yields [`None`]. Nothing else says so, and nothing needs
/// to.
///
/// Which is also all a stream can do about its own failure. There is
/// nowhere to report one after the head has gone — the status is
/// already sent, and HTTP has no way to take it back — so a body that
/// breaks ends, and whoever is reading sees a body that stopped. That
/// is what an ordinary HTTP connection dropping looks like, which is
/// what this is standing in for.
///
/// # It is the same enum a proxy has, written again
///
/// [`McpProxy`](crate::client::mcp_proxy::McpProxy) and
/// [`OciProxy`](crate::client::oci_proxy::OciProxy) each carry one of
/// these, and this is a third copy rather than a shared definition.
/// Deliberately: those two are the CALLER's half and this is the
/// PROVIDER's, they are behind different features, and a type shared
/// across that line would tie two halves together that a build is
/// allowed to compile one of.
pub enum Body {
    /// The whole answer, at once.
    ///
    /// For the ordinary case: one JSON document, complete before it was
    /// sent.
    Single(Bytes),
    /// The answer a piece at a time, for as long as it lasts.
    ///
    /// For an event stream. Each item is another piece of the body, and
    /// the answer is over when the stream ends.
    ///
    /// # Why it is boxed, and why the bounds are what they are
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and will be polled from wherever the answer is being read, which
    /// is not where it was built.
    ///
    /// [`Sync`] is NOT required. Whoever reads the answer OWNS this and
    /// polls it through `&mut`, so a shared reference to it never
    /// exists — and requiring one turns away the obvious way to write a
    /// stream, since an `async_stream` generator is [`Sync`] only if
    /// everything it awaits is. A mutex guard held across an await is
    /// enough to disqualify it.
    Stream(Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>),
}

/// How one HTTP response is written.
///
/// The other half of [`Container::http_serve`]: it hands out requests
/// and one of these each, and this is what an answer goes into.
///
/// # A head, then a body, then a finish
///
/// Which is HTTP's own order and is not a convention imposed here. The
/// [`Head`] carries the status and the headers, so it goes first and
/// goes once; [`body`](Self::body) is called for as much body as there
/// turns out to be, including none.
///
/// It is the same split
/// [`http::response::Frame`](crate::shared::http::response::Frame)
/// makes on the wire, which is what lets an answer be relayed from one
/// to the other without being assembled first.
///
/// # Finishing is a method, not a destructor
///
/// Because a destructor cannot await and cannot report. A response has
/// to be terminated — a chunked body has a terminator, and a reader
/// that does not see one has no way to tell a complete answer from a
/// truncated one — and whether that terminator reached the container is
/// a fact worth having.
///
/// So [`finish`](Self::finish) consumes the writer and says whether it
/// worked. Dropping one without calling it is not a protocol error and
/// nothing here can prevent it; what the container sees is a connection
/// that closed mid-answer, which is what actually happened.
///
/// The same argument this crate makes everywhere it has a choice
/// between a method and a `Drop`.
///
/// [`Head`]: crate::shared::http::response::Head
pub trait HttpResponseWriter {
    /// Why an answer could not be written.
    ///
    /// A provider's own, like everything else here.
    type Error: Send + 'static;

    /// Send the status and the headers.
    ///
    /// Once, before any [`body`](Self::body). What an implementation
    /// does about a second one is its business — there is nothing
    /// useful to do with it, and a type that made it unrepresentable
    /// would be a second writer type for the sake of one mistake.
    fn head(
        &mut self,
        head: crate::shared::http::response::Head,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Send some of the body.
    ///
    /// As many times as there are pieces, and the pieces mean nothing:
    /// an implementation is free to buffer them, and a reader on the
    /// far side will see whatever boundaries the transport produces
    /// rather than these.
    fn body(
        &mut self,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Say the answer is complete.
    ///
    /// See the type's own documentation for why this is a method.
    fn finish(self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// The content for a write ran out early, and this is whose fault it
/// was.
///
/// What an item of the stream
/// [`Container::write`] consumes
/// fails with. It is an enum because a write's content has two origins
/// and they fail in incompatible vocabularies.
///
/// # The two origins
///
/// A caller's write sends its content over the wire, as responses on a
/// channel the provider opened. When that stops early the frame says so
/// with a [`shared::error::Error`](crate::shared::error::Error) — one
/// JSON value, from somebody else's process, meaning whatever that
/// caller meant. That is [`Wire`](Self::Wire).
///
/// A [`transfer`](crate::shared::container::transfer) has no wire in
/// it. Both containers are the provider's, so the bytes go from a
/// [`read`](Container::read) on one straight into a
/// write on the other without ever becoming frames — which is the whole
/// point of having a transfer rather than a read piped through a
/// caller. A read fails with the provider's own error, and that is
/// [`Container`](Self::Container).
///
/// # Why not flatten them
///
/// Making everything a
/// [`shared::error::Error`](crate::shared::error::Error) would mean a
/// provider converting its own read failure into opaque JSON so it
/// could hand it to its own write, in the same process, for nobody's
/// benefit. That is the mistake
/// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
/// exists to avoid: the type that knows most about what happened, lost
/// at the one moment it is in hand.
///
/// Making everything the provider's own error is not available. A
/// caller's failure is a JSON value from another process, and nothing
/// turns one of those into a runtime's error type.
///
/// So neither collapses into the other, and the type says so.
///
/// # It is generic rather than tied to the trait
///
/// [`Container::write`] uses it as
/// `ContentError<Self::Error>`, but nothing here names that. It is a
/// plain two-armed enum over "the wire" and "something else", which is
/// what lets a provider hold one before it has decided which container
/// it is about to write into.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentError<E> {
    /// A caller's content stopped.
    ///
    /// Relayed verbatim from the channel it arrived on, meaning
    /// whatever the caller meant by it. A provider does not read it and
    /// could not usefully — see
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    ///
    /// It is not a failure of the container being written into. The
    /// write is abandoned because there is nothing left to write, not
    /// because anything here went wrong.
    Wire(Error),
    /// Another container's read stopped.
    ///
    /// The provider's own error, from the
    /// [`read`](Container::read) feeding this write.
    /// Which happens for a
    /// [`transfer`](crate::shared::container::transfer), where the
    /// source is a container rather than a caller.
    ///
    /// Whether it is a refused read or a truncated one is not
    /// something a read distinguishes, so this does not either.
    Container(E),
}

impl<E> fmt::Display for ContentError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::Wire(_) => {
                f.write_str("the caller's content stopped")
            }
            ContentError::Container(error) => {
                write!(f, "the source container's read stopped: {error}")
            }
        }
    }
}

impl<E> std::error::Error for ContentError<E>
where
    E: std::error::Error + 'static,
{
    /// [`Wire`](ContentError::Wire) has no source, because what it
    /// carries is not a Rust error and deliberately does not implement
    /// one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContentError::Container(error) => Some(error),
            ContentError::Wire(_) => None,
        }
    }
}
