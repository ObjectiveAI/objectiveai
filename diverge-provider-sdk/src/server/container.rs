//! A container that is running, and what can be done with one.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Sink, Stream};
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};
use serde_json::value::RawValue;

use crate::shared::containers::agentic_loop::response::AgenticLoopChunk;
use crate::shared::error::Error;
use crate::shared::filetree;

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
/// it, for [`mcp_serve`](Self::mcp_serve) — a container's port has a
/// host-side address only on some runtimes, and on the rest the way in
/// is the provider's own control plane, so it is a method and never an
/// address.
///
/// [`filetree`](Self::filetree) is among them for the same reason:
/// watching a filesystem is knowing where one is. A transfer is not —
/// it is a read on one container and a write on another, both already
/// here.
///
/// # Ports are exchanges, not one pipe
///
/// There was a `connect` that handed back a byte pipe, and everything
/// spoken to a container rode one. It is gone, because the things
/// riding it were not one thing.
///
/// MCP goes both ways and they are not the same method.
/// [`mcp_serve`](Self::mcp_serve) is the provider ANSWERING: a
/// container's own MCP client asks, and what it is asking for lives
/// with the caller. The five `mcp_` methods are the provider ASKING,
/// into an MCP server the container runs itself. An agentic loop needs
/// the first; a plugin and a laboratory need the second — a
/// laboratory's own asks of the caller are an image, an authorization
/// and a write's content, none of which is MCP.
///
/// [`agentic_loop`](Self::agentic_loop) is neither, being the
/// one thing a container says that this crate defined — so it is the
/// one thing read rather than relayed.
///
/// [`command_serve`](Self::command_serve) is the provider taking asks
/// again — a plugin's CLI commands, each one ask and a stream of
/// answers. Typed as far as the asking goes and opaque inside, because
/// what a command says belongs to the CLI.
///
/// [`postgres_serve`](Self::postgres_serve) is the exception, and it is
/// the only one still shaped like a socket — because pgwire is a duplex
/// conversation rather than a series of exchanges, and there was
/// nothing in it to hand over instead.
///
/// Which is why the pipe was the wrong shape for all of them. It was
/// built for the one case that needs it, and the rest were made to
/// speak through a socket when what they had was exchanges.
///
/// # HTTP is the implementation's, and is named nowhere here
///
/// A container really is an HTTP server — an MCP one, reached over
/// Streamable HTTP. This trait used to say so, handing over parsed
/// requests and statuses and header maps, and every one of those was a
/// chance to relay something wrong: a `Content-Length` copied onto a
/// body that had been re-encoded, a `Connection` forwarded past the hop
/// it belonged to.
///
/// So the exchanges are typed and the transport is not mentioned. An
/// implementation is an MCP client on one port and an MCP server on
/// another, which is what rmcp is for, and what it does with sockets is
/// its own business.
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
    /// One type for every method that can fail, because a provider
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
    /// One item per request, each with the [`McpResponder`] that
    /// answers that one. The stream ends when the container has no more
    /// to ask — see [`mcp_serve`](Self::mcp_serve).
    ///
    /// # The request is typed, where it used to be bytes
    ///
    /// It was an encoded HTTP request, forwarded onward without being
    /// read, because the wire carried the same thing and a relay that
    /// decoded one only to encode it again would be taking a request
    /// apart to prove it could be.
    ///
    /// The wire carries the exchanges themselves now — a
    /// [`CallToolRequestParams`] rather than a `POST` containing one —
    /// so there is nothing left to pass through. What replaces the
    /// saved re-encoding is a whole class of relay bug that cannot be
    /// written: the request that goes out is the request that arrived,
    /// because they are the same value.
    ///
    /// # It is a pair rather than a request that answers itself
    ///
    /// Because the answer travels a different way than the ask. An
    /// implementation holds whatever it needs to write back — a
    /// connection, a stream id, a channel into its own server — and
    /// that is the responder, not something this crate could put inside
    /// an enum it defines.
    type McpRequestStream: Stream<Item = (McpRequest, Self::McpResponder)>
        + Send
        + Unpin
        + 'static;

    /// Where one of those requests is answered.
    ///
    /// Its error is [`Self::Error`] because a failure to answer is a
    /// failure of the same connection the request arrived on, and
    /// splitting them would be inventing a distinction a provider does
    /// not have.
    type McpResponder: McpResponder<Error = Self::Error> + Send + 'static;

    /// What a container's own MCP server says on its own account.
    ///
    /// What [`mcp_notifications`](Self::mcp_notifications) hands back:
    /// tools changed, resources changed, a resource updated, a log
    /// line, for as long as the container has any.
    ///
    /// An item that is [`Err`] is the container's server saying it will
    /// push no more, and why. Nothing follows one — which is the
    /// server's own vocabulary and not this crate's, so it is an
    /// [`ErrorData`] rather than a [`Self::Error`].
    type McpNotificationsStream: Stream<
            Item = Result<ServerNotification, ErrorData>,
        > + Send
        + Unpin
        + 'static;

    /// What an agent says, as it says it.
    ///
    /// One item per chunk, already parsed. See
    /// [`agentic_loop`](Self::agentic_loop) for why this is
    /// the one thing a container says that arrives as something other
    /// than bytes.
    ///
    /// An item that is [`Err`] is an event that could not be read, and
    /// it does not end the stream: it is one thing the agent said that
    /// this crate could not, and the next one may be fine. Which is
    /// what keeps "the agent said something unreadable" from looking
    /// like "the agent finished".
    type AgenticLoopStream: Stream<
            Item = Result<AgenticLoopChunk, Self::Error>,
        > + Send
        + Unpin
        + 'static;

    /// The database connections a container opens, as it opens them.
    ///
    /// One item per connection, and nothing before it. A plugin holds a
    /// POOL, so this is the shape that says how many there are: however
    /// many turn up.
    type PostgresConnectionStream: Stream<
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

    /// The commands a container asks to have run, as it asks.
    ///
    /// One item per command: the whole ask, beside the
    /// [`CommandWriter`](Self::CommandWriter) that carries its answers
    /// back in.
    ///
    /// # The ask arrives whole
    ///
    /// The container-side protocol ends an ask with a half-close — see
    /// [`command_serve`](Self::command_serve) for the whole of it — so
    /// reading up to one is transport framing, and framing is the
    /// implementation's. The same doctrine that put SSE reassembly
    /// inside [`agentic_loop`](Self::agentic_loop): the thing that
    /// knows how the pieces were cut is the thing that puts them back
    /// together, and a consumer is handed an ask and never a piece of
    /// one.
    type CommandStream: Stream<Item = (Bytes, Self::CommandWriter)>
        + Send
        + Unpin
        + 'static;

    /// Where one command's answers go.
    ///
    /// Items as they arrive, opaque. What a command produces belongs to
    /// the CLI, which gains subcommands on its own schedule, and
    /// nothing between the caller and the plugin reads one.
    ///
    /// # Dropping it is the finish
    ///
    /// The pipe closing is how the plugin learns the command is over,
    /// and dropping this is what closes it. There is no method for the
    /// ending because there is nothing to distinguish: unlike a
    /// notification stream, which can end well or badly and has an
    /// error frame to say which, a command's answers just stop — the
    /// pipe closing is the whole vocabulary the plugin has, whatever
    /// ended them.
    type CommandWriter: Sink<Bytes, Error = Self::Error>
        + Send
        + Unpin
        + 'static;

    /// The container's filesystem, as it changes.
    ///
    /// What [`filetree`](Self::filetree) hands back: one
    /// [`Snapshot`](filetree::response::Frame::Snapshot) first, then
    /// one frame per change, for as long as the stream is held.
    ///
    /// # The items are infallible, and the ending is the failure
    ///
    /// A watch that breaks mid-run simply ends the stream. The
    /// container is still fine — a broken watch says nothing about the
    /// filesystem it was watching — so whoever was reading keeps a
    /// stale tree and everything else keeps working. There is no error
    /// to carry because there is nothing a consumer could do with one
    /// that ending does not already say.
    type FiletreeStream: Stream<Item = filetree::response::Frame>
        + Send
        + Unpin
        + 'static;

    /// Serve MCP to something inside the container.
    ///
    /// The container is the CLIENT here. It asks and this end answers,
    /// which is the direction an agent's tool calls travel: an
    /// [`agent container`](crate::endpoints::containers::agents) runs
    /// its agent beside the provider and the MCP servers live with the
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
    /// # Exchanges, not a byte pipe
    ///
    /// The implementation speaks MCP and this crate speaks the wire.
    /// Which is the whole point of the shape: a pipe would have every
    /// consumer parsing request heads and decoding chunked bodies, and
    /// there is no version of that which is this protocol's business.
    ///
    /// What arrives is one exchange per item, already separated from
    /// the next and already read. Where one ends is MCP's question and
    /// HTTP's beneath it, answered by neither this crate nor its
    /// consumers.
    ///
    /// # Answers can be given in any order
    ///
    /// Each request arrives with its own responder, and nothing pairs a
    /// responder with the one that came before it. A consumer that
    /// takes three requests and answers the third first has done
    /// nothing wrong, and an implementation has to be able to carry
    /// that — which for HTTP/1.1 means a connection each, and for
    /// anything newer means a stream each.
    ///
    /// # The stream ends when the container stops asking
    ///
    /// Cleanly, and it says nothing about the container. An agent that
    /// has no more tool calls to make and an agent that has finished
    /// its work look the same from here, because they are the same
    /// thing from here: the conversation is over and the container's
    /// own life is [`stop`](Self::stop)'s business.
    ///
    /// An [`Err`] is a failure to START serving — nothing listening on
    /// that port, or a port that was never declared. A failure after
    /// that ends the stream, since a request that cannot be received is
    /// indistinguishable from one that was never sent.
    fn mcp_serve(
        &self,
        port: u16,
    ) -> impl Future<
        Output = Result<Self::McpRequestStream, Self::Error>,
    > + Send;

    /// Ask what tools the container offers.
    ///
    /// [`None`] asks for the first page. A server with more to give
    /// says so with a cursor, and the next page is another call.
    ///
    /// # Two failures, nested, and they are different facts
    ///
    /// The outer [`Err`] is this crate not reaching the container:
    /// nothing listening on that port, a port never declared, a
    /// connection that broke. The inner one is the container's own MCP
    /// server refusing, in its own vocabulary, with a JSON-RPC code
    /// that means something.
    ///
    /// Flattening them would make "the port is wrong" and "no such
    /// tool" the same answer, and only one of those is about the
    /// plugin. It is the rule the whole trait follows: what the thing
    /// inside SAID is an [`Ok`], including when what it said was no.
    fn mcp_list_tools(
        &self,
        port: u16,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<
        Output = Result<Result<ListToolsResult, ErrorData>, Self::Error>,
    > + Send;

    /// Ask what resources the container offers.
    ///
    /// The same shape [`mcp_list_tools`](Self::mcp_list_tools) has, for
    /// the same reason: it is the same MCP request against a different
    /// noun.
    ///
    /// # Two failures, nested, and they are different facts
    ///
    /// The outer [`Err`] is this crate not reaching the container:
    /// nothing listening on that port, a port never declared, a
    /// connection that broke. The inner one is the container's own MCP
    /// server refusing, in its own vocabulary, with a JSON-RPC code
    /// that means something.
    ///
    /// Flattening them would make "the port is wrong" and "no such
    /// tool" the same answer, and only one of those is about the
    /// plugin. It is the rule the whole trait follows: what the thing
    /// inside SAID is an [`Ok`], including when what it said was no.
    fn mcp_list_resources(
        &self,
        port: u16,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<
        Output = Result<Result<ListResourcesResult, ErrorData>, Self::Error>,
    > + Send;

    /// Run one of the container's tools.
    ///
    /// A tool that FAILS is still an inner [`Ok`]:
    /// [`CallToolResult`] carries its own `is_error`, which is a tool
    /// saying its work did not succeed. An inner [`Err`] is the server
    /// refusing to run it at all.
    ///
    /// # Two failures, nested, and they are different facts
    ///
    /// The outer [`Err`] is this crate not reaching the container:
    /// nothing listening on that port, a port never declared, a
    /// connection that broke. The inner one is the container's own MCP
    /// server refusing, in its own vocabulary, with a JSON-RPC code
    /// that means something.
    ///
    /// Flattening them would make "the port is wrong" and "no such
    /// tool" the same answer, and only one of those is about the
    /// plugin. It is the rule the whole trait follows: what the thing
    /// inside SAID is an [`Ok`], including when what it said was no.
    fn mcp_call_tool(
        &self,
        port: u16,
        params: CallToolRequestParams,
    ) -> impl Future<
        Output = Result<Result<CallToolResult, ErrorData>, Self::Error>,
    > + Send;

    /// Read one of the container's resources.
    ///
    /// By URI, which is the container's to interpret. Nothing between
    /// here and it resolves one.
    ///
    /// # Two failures, nested, and they are different facts
    ///
    /// The outer [`Err`] is this crate not reaching the container:
    /// nothing listening on that port, a port never declared, a
    /// connection that broke. The inner one is the container's own MCP
    /// server refusing, in its own vocabulary, with a JSON-RPC code
    /// that means something.
    ///
    /// Flattening them would make "the port is wrong" and "no such
    /// tool" the same answer, and only one of those is about the
    /// plugin. It is the rule the whole trait follows: what the thing
    /// inside SAID is an [`Ok`], including when what it said was no.
    fn mcp_read_resource(
        &self,
        port: u16,
        params: ReadResourceRequestParams,
    ) -> impl Future<
        Output = Result<Result<ReadResourceResult, ErrorData>, Self::Error>,
    > + Send;

    /// Hear what the container's own MCP server says unprompted.
    ///
    /// The fifth of the inward asks, and the one that is not answered
    /// once. The other four ask and are told; this opens the place a
    /// server pushes into and reads it for as long as it is held.
    ///
    /// # It takes nothing
    ///
    /// Because in MCP there is nothing to ask: a client opens that
    /// stream with a bare `GET` and no body, and there is no
    /// `notifications/subscribe` to mirror.
    ///
    /// Dropping the stream is how this end says it has stopped
    /// listening. There is nothing to unsubscribe with and nothing
    /// needs one.
    ///
    /// The [`Err`] here is the same outer one the four have: no stream
    /// at all, because the port was wrong or nothing was listening. A
    /// server that refuses says so inside the stream — see
    /// [`McpNotificationsStream`](Self::McpNotificationsStream).
    fn mcp_notifications(
        &self,
        port: u16,
    ) -> impl Future<
        Output = Result<Self::McpNotificationsStream, Self::Error>,
    > + Send;

    /// Start an agentic loop, and take the chunks it produces.
    ///
    /// The one exchange whose answer is read rather than relayed. What
    /// comes back is what the agent said, in the shape this crate
    /// defines for it.
    ///
    /// # It is the one thing here that is not somebody else's protocol
    ///
    /// Everything else a container says passes through. An MCP server's
    /// answers belong to MCP, a registry's to the registry API, a
    /// command's to the CLI — and this trait carries all of them as
    /// bytes because it has no standing to interpret them, and a relay
    /// that parsed could only drop what its schema was too old to know.
    ///
    /// An agentic loop is not passing through. The image producing it
    /// is this crate's, the
    /// [`AgenticLoopChunk`] it produces is this crate's, and the caller
    /// receives that same type at the far end. There is no third party
    /// whose protocol would be being second-guessed.
    ///
    /// So this is where the line falls: a container's answer is raw
    /// unless this crate is the thing that defined it.
    ///
    /// # Where the framing went
    ///
    /// Into the implementation, which is the only place that can do it.
    /// An agent answers with an event stream, and the pieces an HTTP
    /// body arrives in are not the events it contains — so something
    /// has to hold the leftovers and hand over a chunk each time a
    /// whole one is there.
    ///
    /// That something knows it is reading an event stream, because it
    /// read the head — and it is the only thing that does. A consumer
    /// handed the body would be reassembling from pieces, having been
    /// told nothing about how they were cut.
    ///
    /// # No head, and no unary case
    ///
    /// This promises chunks, so anything that is not chunks is a
    /// failure to produce them: a status that is not a success, a
    /// response that is not an event stream, a connection that broke
    /// before one started. All of it is the [`Err`], and a caller that
    /// gets [`Ok`] has a stream and nothing left to check.
    ///
    /// Which is what makes the missing head not a loss. There is
    /// nothing to judge — the judging already happened, and the answer
    /// was either a stream of chunks or it was not.
    ///
    /// A run that produces one chunk is a stream of one. Nothing else
    /// would be simpler: a caller reading a stream reads the same code
    /// either way, where a caller choosing between two shapes writes
    /// the choice out every time.
    /// # What goes in is the caller's own body
    ///
    /// Borrowed as the [`RawValue`] it arrived as, rather than
    /// re-serialized out of a decoder, so a field this crate does not
    /// model survives the trip. What an implementation wraps it in —
    /// which verb, which path, which headers — is between it and the
    /// image, and neither is this protocol's.
    fn agentic_loop(
        &self,
        port: u16,
        body: &RawValue,
    ) -> impl Future<
        Output = Result<Self::AgenticLoopStream, Self::Error>,
    > + Send;

    /// Take the database connections a container opens.
    ///
    /// A container that dials Postgres has a database, and the
    /// database lives with the CALLER. So the
    /// plugin connects, and every connection it opens has to be carried
    /// out of the container and offered to whoever holds the data.
    ///
    /// # It is bytes, and it is the one thing that has to be
    ///
    /// Everything else spoken to a container is an exchange with a
    /// beginning and an end, which is what lets
    /// [`mcp_serve`](Self::mcp_serve) and the five `mcp_` methods hand
    /// over asks and answers instead of a socket. This cannot be:
    /// pgwire is a duplex conversation with its own framing, its own
    /// pipelining, and messages that arrive unprompted, and there is no
    /// exchange in it to hand over.
    ///
    /// Parsing it would mean tracking it — a wire protocol that gains
    /// messages on somebody else's schedule, inside the crate that is
    /// this protocol's normative artifact. So the bytes go through
    /// unread, which is also what the caller's
    /// `PostgresProxy`
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
    ) -> impl Future<
        Output = Result<Self::PostgresConnectionStream, Self::Error>,
    > + Send;

    /// Take commands as the container asks for them.
    ///
    /// A plugin may want things run that only the caller can run — the
    /// caller's own CLI, against the caller's own state — so it asks,
    /// and the ask has to leave the container before anything can
    /// happen. What the caller's
    /// `CommandProxy`
    /// produces comes back through the item's writer.
    ///
    /// # The container-side protocol, whole
    ///
    /// On the port named here, one connection is one command: the
    /// plugin writes the ask, half-closes, reads the answers, and the
    /// pipe closing is the command being over. Nothing delimits the ask
    /// because nothing has to — the half-close does — and nothing leads
    /// it. There was a four-byte exchange id in front once, and it was
    /// read by nobody ever: one pipe is one command, so the pipe itself
    /// is the correlation.
    ///
    /// How an implementation keeps connections available to a plugin
    /// that listens is its own business, the way sockets are its
    /// business everywhere else here.
    ///
    /// # Ends and errors
    ///
    /// The stream ends when the plugin stops asking, which says nothing
    /// about the container: a plugin between commands and a plugin that
    /// has finished look identical from here, because they are
    /// identical from here.
    ///
    /// An [`Err`] is a failure to START — nothing listening on that
    /// port, or a port never declared. A failure after that ends the
    /// stream. A failure on one COMMAND is that command's: its writer
    /// errors, and dropping the writer closes the pipe, which is all
    /// the plugin can be told either way.
    fn command_serve(
        &self,
        port: u16,
    ) -> impl Future<
        Output = Result<Self::CommandStream, Self::Error>,
    > + Send;

    /// Watch the container's filesystem.
    ///
    /// What a laboratory reports for its whole life: the observable
    /// part of a container running is its filesystem, so the scope that
    /// made the container is the scope that reports on it — and this is
    /// where the reporting comes from.
    ///
    /// # Every call is a fresh subscription
    ///
    /// One snapshot, then deltas, PER CALL. A connector arriving an
    /// hour into a run needs the whole tree before any change to it
    /// means anything, and the runner's own stream is an hour past its
    /// snapshot — so each observer asks for its own, and an
    /// implementation multiplexes one watch into as many subscriptions
    /// as are held. How it does that is its business; that each stream
    /// begins whole is the contract.
    ///
    /// # No port, and no path
    ///
    /// Watching a filesystem is knowing where one is, which is exactly
    /// the fact this trait exists to hold. The tree is the container's
    /// root; what a provider leaves out of it — mounts being the case
    /// worth knowing about, being somebody else's filesystem reached
    /// across a boundary that carries no change notifications — is
    /// covered on the endpoint's own response.
    ///
    /// An [`Err`] is a failure to START: the watch could not be set up
    /// at all. A failure after that ends the stream — see
    /// [`FiletreeStream`](Self::FiletreeStream) for why that is the
    /// whole of it.
    fn filetree(
        &self,
    ) -> impl Future<
        Output = Result<Self::FiletreeStream, Self::Error>,
    > + Send;

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
    /// See [`read`](crate::shared::containers::read) for why. A path
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
    /// reference a [`filetree`] stream uses.
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
    /// `transfer`'s comes from a
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

/// One thing a container's MCP client asked for.
///
/// The item of [`Container::McpRequestStream`], paired with the
/// [`McpResponder`] that answers it.
///
/// # Five, because the wire carries five
///
/// They are the exchanges an
/// [`agent container`](crate::endpoints::containers::agents) relays
/// outward and nothing else. MCP has more — `initialize`, `ping`,
/// `complete`, `subscribe` — and none of them crosses this connection:
/// initialization is between the container and whatever serves it, and
/// the rest were never in the protocol this crate defines.
///
/// So an implementation that receives one of those answers it itself or
/// refuses it. It never arrives here, because there is no variant for
/// it to arrive as.
///
/// # It carries the params and not the request
///
/// Which is what an rmcp `CallToolRequest` would have added: a `method`
/// that is a constant, and a `jsonrpc` that is a constant. Both are
/// framing, both are the implementation's, and neither survives being
/// put on a wire that already knows which exchange this is — the
/// variant says it.
#[derive(Debug, Clone, PartialEq)]
pub enum McpRequest {
    /// What tools have you got.
    ///
    /// [`None`] is the first page. A cursor asks for another.
    ListTools(Option<PaginatedRequestParams>),
    /// What resources have you got.
    ListResources(Option<PaginatedRequestParams>),
    /// Run this tool.
    CallTool(CallToolRequestParams),
    /// Read this resource.
    ReadResource(ReadResourceRequestParams),
    /// Tell me what happens.
    ///
    /// It carries nothing because there is nothing to ask: a client
    /// opens that stream with a bare `GET` and no body.
    ///
    /// It is also the only variant whose answer is not one thing. See
    /// [`McpResponder::notification`], which is written as many times
    /// as there are notifications.
    Notifications,
}

/// How one of those is answered.
///
/// The other half of [`Container::mcp_serve`]: it hands out requests
/// and one of these each, and this is where an answer goes.
///
/// # A method per variant, and the pairing is not enforced
///
/// Nothing stops a [`ListTools`](McpRequest::ListTools) being answered
/// with [`call_tool`](Self::call_tool). The alternative was a responder
/// type per variant, so that the result type is fixed by the request
/// that arrived — five associated types on [`Container`], to prevent a
/// mistake in the one place that constructs these and matches on them
/// two lines earlier.
///
/// # Four consume, one does not
///
/// [`list_tools`](Self::list_tools),
/// [`list_resources`](Self::list_resources),
/// [`call_tool`](Self::call_tool) and
/// [`read_resource`](Self::read_resource) take `self`, because an
/// exchange answered once cannot be answered again — and taking `self`
/// is how that stops being a rule and starts being a fact. Answering
/// them IS finishing them; there is nothing left to terminate.
///
/// [`notification`](Self::notification) takes `&mut self` and is called
/// for as long as there is anything to say, then
/// [`notifications_finish`](Self::notifications_finish) ends it.
///
/// # Finishing is a method, not a destructor
///
/// Because a destructor cannot await and cannot report. A notification
/// stream has to be terminated — a reader that never sees the end has
/// no way to tell a server that finished from a connection that
/// dropped — and whether that terminator reached the container is a
/// fact worth having.
///
/// Dropping one without finishing it is not a protocol error and
/// nothing here can prevent it; what the container sees is a connection
/// that closed mid-answer, which is what actually happened.
///
/// The same argument this crate makes everywhere it has a choice
/// between a method and a `Drop`.
pub trait McpResponder {
    /// Why an answer could not be written.
    ///
    /// A provider's own, like everything else here.
    type Error: Send + 'static;

    /// Answer a [`ListTools`](McpRequest::ListTools).
    fn list_tools(
        self,
        result: Result<ListToolsResult, ErrorData>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Answer a [`ListResources`](McpRequest::ListResources).
    fn list_resources(
        self,
        result: Result<ListResourcesResult, ErrorData>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Answer a [`CallTool`](McpRequest::CallTool).
    ///
    /// An [`Err`] is the tool not being run — no such tool, arguments
    /// that do not match its schema. A tool that ran and failed is an
    /// [`Ok`] whose [`CallToolResult`] says so, which is MCP's
    /// distinction and this keeps it.
    fn call_tool(
        self,
        result: Result<CallToolResult, ErrorData>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Answer a [`ReadResource`](McpRequest::ReadResource).
    fn read_resource(
        self,
        result: Result<ReadResourceResult, ErrorData>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Push one notification onto a
    /// [`Notifications`](McpRequest::Notifications).
    ///
    /// As many times as there are notifications, and then
    /// [`notifications_finish`](Self::notifications_finish).
    fn notification(
        &mut self,
        notification: ServerNotification,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Say a [`Notifications`](McpRequest::Notifications) is over.
    ///
    /// Named for its variant rather than called `finish`, because the
    /// other four have nothing to finish: they are answered once and
    /// answering is the end of them.
    ///
    /// # The error is the far server's, and it belongs here
    ///
    /// [`None`] is a stream that ended with nothing more to say.
    /// [`Some`] is the caller's MCP server saying it will push no more,
    /// and why — which is a thing that channel can carry, and a
    /// container told only that there is no more could not tell a
    /// server that finished from one that broke.
    fn notifications_finish(
        self,
        error: Option<ErrorData>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
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
/// A `transfer` has no wire in
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
    /// `transfer`, where the
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
