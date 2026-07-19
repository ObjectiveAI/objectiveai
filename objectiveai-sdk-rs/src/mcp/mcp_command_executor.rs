//! Command execution over an MCP connection — the fulfilling side.
//!
//! An MCP connection may be established with the optional
//! command-execution extension: the client declares at connect time
//! that it is willing to fulfill CLI command requests the server sends
//! back over the connection's standing SSE stream. When a command
//! request arrives on that stream, the connection hands the request to
//! its [`McpCommandExecutor`] implementor and pumps the resulting stream
//! back to the server — one POST per item, in order, then a terminal
//! frame.
//!
//! This trait is the mirror of the `cli::command::CommandExecutor`
//! trait, and deliberately not the same trait: `CommandExecutor` is the
//! REQUESTING side (mint a request, consume the result stream — its
//! generics let the caller pick typed request/response leaves), while
//! `McpCommandExecutor` is the FULFILLING side (run a request that
//! arrived off the wire). The connection is a transport, so everything
//! here is wire-shaped [`serde_json::Value`]s: the `mcp` feature cannot
//! see the `cli` feature's types (`cli` depends on `mcp`, not the
//! reverse), and the connection never interprets the payloads it
//! carries — implementors deserialize the request into the typed
//! `cli.command.Request` themselves and yield items already shaped for
//! the wire.

/// Fulfills CLI command requests arriving over an MCP connection's SSE
/// stream.
///
/// Implementors run the request against some backend (for the daemon:
/// its in-process command machinery, with caller identity baked into
/// the implementor instance at connect time — identity never rides the
/// wire) and surface the output as a stream of wire-shaped JSON items.
///
/// `Send + Sync + 'static` supertraits: the connection holds the
/// implementor inside its `Arc`'d inner state and calls it from the
/// SSE listener task, so every implementor must already be shareable
/// across tasks.
pub trait McpCommandExecutor: Send + Sync + 'static {
    /// Failure to start a run, or a per-item failure on the stream.
    /// `Display` is required (unlike `cli::command::CommandExecutor`,
    /// whose caller consumes errors natively) because the connection
    /// must encode errors onto the wire to report them to the server.
    type Error: std::fmt::Display + Send + 'static;

    /// The item stream for one command run. Items are wire-shaped JSON
    /// values the connection POSTs back to the server verbatim.
    type Stream: futures_util::Stream<Item = Result<serde_json::Value, Self::Error>>
        + Send
        + 'static;

    /// Run one command request. `request` is the serialized
    /// `cli.command.Request` JSON exactly as it arrived off the wire.
    ///
    /// Returning `Err` means the run could not start (deserialization
    /// failure, gate rejection, …); item-level failures after a
    /// successful start ride the stream instead. The stream ending is
    /// the end of the run — dropping the stream before it ends cancels
    /// the run.
    fn execute(
        &self,
        request: serde_json::Value,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send;
}

/// The [`McpCommandExecutor`] for connections established WITHOUT the
/// command-execution extension.
///
/// A connection that never declared the extension at connect time can
/// never be routed a command request by the server, so this
/// implementation's `execute` is statically dead code — it panics with
/// `unreachable!` rather than returning an error, because reaching it
/// is a connection-layer bug (routing a request to a connection that
/// never offered to fulfill them), not a runtime condition to handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnimplementedMcpCommandExecutor;

impl McpCommandExecutor for UnimplementedMcpCommandExecutor {
    type Error = std::convert::Infallible;
    type Stream = futures_util::stream::Empty<Result<serde_json::Value, Self::Error>>;

    async fn execute(
        &self,
        _request: serde_json::Value,
    ) -> Result<Self::Stream, Self::Error> {
        unreachable!(
            "UnimplementedMcpCommandExecutor::execute: this connection was \
             established without the command-execution extension, so the \
             server can never route a command request to it"
        )
    }
}
