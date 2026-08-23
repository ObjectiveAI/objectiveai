//! Answering the MCP exchanges a provider forwards.

use std::future::Future;
use std::pin::Pin;

use futures_util::Stream;
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};

/// What answers the MCP exchanges a provider cannot.
///
/// A provider opens an MCP channel because something inside a container
/// wants to talk to a server the provider cannot reach — the MCP
/// servers live with the caller. So the ask comes out, and this is what
/// a caller implements to answer it.
///
/// # Five methods, because there are five exchanges
///
/// One each for what
/// [`shared::mcp`](crate::shared::mcp) defines: listing tools, listing
/// resources, calling a tool, reading a resource, and hearing what a
/// server says on its own account.
///
/// There was one method, taking a tunneled HTTP request. That worked
/// because the request said which exchange it was — a method and a path
/// and a JSON-RPC body an implementation had to parse to find out. Now
/// the channel says it, so there is no "an MCP request" left to take.
///
/// # It is still a proxy, and less of one than it was
///
/// Nothing here tracks a session or reads the `Mcp-Session-Id` that
/// used to tie a caller's exchanges together, because there is no
/// header to read: a channel is the session, opened by one container
/// inside one scope.
///
/// What DID change is that an implementation now knows which of five
/// things it was asked. That is not this crate interpreting MCP — it is
/// this crate no longer pretending it was carrying something opaque
/// when the far end had to parse it anyway.
///
/// # The params are [`rmcp`]'s
///
/// Not the [`Request`](crate::shared::mcp::call_tool::request::Request)
/// wrappers beside them. Those exist so a frame can encode one, and
/// that job is finished by the time this is called — handing over a
/// wrapper here would be asking every implementation to unwrap
/// something for no reason, when what it wants is the type its own MCP
/// client already takes.
///
/// # Failure is an [`ErrorData`], where it used to be a status
///
/// The old shape had no [`Result`] at all: the exchange was HTTP, so a
/// proxy that could not reach its server answered `502` and one asked
/// for something absent answered `404`. There was always an answer, and
/// only what it said varied.
///
/// There is no HTTP left to carry a status, and MCP has its own way of
/// saying no — a JSON-RPC error, with a code that means something.
/// `-32601` is "no such tool" and `-32602` is "the arguments were
/// wrong", and an agent told only that something failed can act on
/// neither.
///
/// So an [`Err`] is what an MCP server would have replied, relayed. It
/// is [`rmcp`]'s type rather than
/// [`shared::error::Error`](crate::shared::error::Error) because the
/// failure is not the caller's own — the caller is passing on somebody
/// else's, and the code is the content.
pub trait McpProxy: Send + Sync {
    /// What tools are there.
    ///
    /// [`None`] asks for the first page. A server with more to give
    /// says so with a cursor, and the next page is another call.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider serving several containers answers their
    /// exchanges at once, and a task that cannot move between threads
    /// pins that concurrency to one. It is spelled out rather than left
    /// to `async fn`, which promises nothing about the future it
    /// returns. The same holds for all five.
    fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send;

    /// What resources are there.
    ///
    /// The same shape [`list_tools`](Self::list_tools) has, for the
    /// same reason: it is the same MCP request against a different
    /// noun.
    fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + Send;

    /// Run one tool.
    ///
    /// # A tool that fails is not an [`Err`]
    ///
    /// [`CallToolResult`] carries its own `is_error`, which is a tool
    /// saying its work did not succeed — a compile that failed, a query
    /// that found nothing. That is an answer and travels as [`Ok`].
    ///
    /// An [`Err`] is the server refusing to run it at all: no such
    /// tool, arguments that do not match its schema, a server that
    /// broke. The distinction is MCP's rather than this crate's, and
    /// keeping it is most of why the error type is [`rmcp`]'s.
    fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send;

    /// Read one resource.
    ///
    /// By URI, which is the server's to interpret. A proxy that
    /// resolved one would be deciding what a resource is.
    fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> impl Future<Output = Result<ReadResourceResult, ErrorData>> + Send;

    /// Everything the server says on its own account.
    ///
    /// Tools changed, resources changed, a resource updated, a log
    /// line. See
    /// [`ServerNotification`] for the whole of what one can be.
    ///
    /// # It takes nothing
    ///
    /// Alone among the five, because in MCP there is nothing to ask.
    /// The other four are JSON-RPC methods and carry their params; this
    /// is not a method — a client opens the notification stream with a
    /// bare `GET` on the same url it POSTs everything else to, with no
    /// body. There is no `notifications/subscribe` to mirror.
    ///
    /// # It returns a stream, and the stream is the answer
    ///
    /// The other four resolve to a value. This resolves to something
    /// that keeps producing for as long as it is held, which is what a
    /// notification stream is: not an answer, but the place a server
    /// pushes into when something changes.
    ///
    /// It ends when the server has no more to say, and dropping it is
    /// how a provider says it has stopped listening. There is no
    /// unsubscribe, because the channel ending is one.
    ///
    /// # An [`Err`] item is the last one
    ///
    /// A stream that stops is a stream that stopped, and a caller that
    /// can no longer keep one open says why rather than ending in
    /// silence. Nothing follows it.
    ///
    /// # Why it is boxed, and why the bounds are what they are
    ///
    /// [`Send`] and `'static` because it outlives the call that made it
    /// and will be polled from wherever the answer is being written,
    /// which is not where it was built.
    ///
    /// [`Sync`] is NOT required. Whoever writes the answer OWNS this
    /// and polls it through `&mut`, so a shared reference to it never
    /// exists — and requiring one turns away the obvious way to write a
    /// stream, since an `async_stream` generator is [`Sync`] only if
    /// everything it awaits is.
    fn notifications(
        &self,
    ) -> impl Future<
        Output = Pin<
            Box<
                dyn Stream<Item = Result<ServerNotification, ErrorData>>
                    + Send
                    + 'static,
            >,
        >,
    > + Send;
}
