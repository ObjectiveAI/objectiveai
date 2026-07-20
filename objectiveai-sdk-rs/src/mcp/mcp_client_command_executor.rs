//! Command execution over an MCP connection — the fulfilling side.
//!
//! An MCP server may expose the objectiveai capability (the
//! [`OBJECTIVEAI_CAPABILITY`] key in `ServerCapabilities.experimental`).
//! When it does, it may push
//! `notifications/objectiveai/cli_request` frames over the connection's
//! standing SSE stream, asking this CLIENT to run a CLI command. The
//! connection fulfills requests IN PARALLEL (each spawned off the
//! listener; frame order is guaranteed per run, not across runs) —
//! by running each through its [`McpClientCommandExecutor`] and POSTing
//! every resulting item to the server's
//! `{mcp_url}/objectiveai/command` endpoint
//! ([`CLI_COMMAND_ENDPOINT_SUFFIX`]) as a [`CliResponse`] frame
//! carrying the request's correlation id, as it arrives. Stream errors
//! are NON-terminal ([`CliResponse::Error`]); the exchange ALWAYS ends
//! with a [`CliResponse::Done`] — even when the run failed to start.
//!
//! This trait is the mirror of the
//! [`cli::command::CommandExecutor`](crate::cli::command::CommandExecutor)
//! trait, and deliberately not the same trait: `CommandExecutor` is the
//! REQUESTING side (mint a request, consume the result stream — its
//! generics let the caller pick typed request/response leaves), while
//! `McpClientCommandExecutor` is the FULFILLING side (run a request
//! that arrived off the wire) — named from the perspective of the MCP
//! CLIENT, which is the party that executes commands the MCP server
//! requests. Everything is typed end to end: requests come in as
//! [`cli::command::Request`](crate::cli::command::Request), items go
//! out as
//! [`cli::command::ResponseItem`](crate::cli::command::ResponseItem).

use schemars::JsonSchema;

/// The key an MCP server sets in `ServerCapabilities.experimental` to
/// declare the objectiveai command-execution extension. Presence of
/// the key is the capability; its value is reserved for future
/// extension settings.
pub const OBJECTIVEAI_CAPABILITY: &str = "objectiveai";

/// Path suffix appended to the connection's MCP endpoint URL to form
/// the command-response endpoint the connection POSTs [`CliResponse`]
/// frames to: `{mcp_url}/objectiveai/command`.
pub const CLI_COMMAND_ENDPOINT_SUFFIX: &str = "/objectiveai/command";

/// Fulfills CLI command requests arriving over an MCP connection's SSE
/// stream.
///
/// Implementors run the request against some backend (for the daemon:
/// its in-process command machinery, with caller identity baked into
/// the implementor instance at connect time — identity never rides the
/// wire) and surface the output as a stream of typed response items.
///
/// `Send + Sync + 'static` supertraits: the connection holds the
/// implementor inside its `Arc`'d inner state and calls it from the
/// SSE listener task, so every implementor must already be shareable
/// across tasks.
pub trait McpClientCommandExecutor: Send + Sync + 'static {
    /// Failure to start a run, or a per-item failure on the stream.
    /// `Display` is required (unlike `cli::command::CommandExecutor`,
    /// whose caller consumes errors natively) because the connection
    /// must encode errors onto the wire ([`CliResponse::Error`]) to
    /// report them to the server.
    type Error: std::fmt::Display + Send + 'static;

    /// The item stream for one command run — the typed CLI response
    /// items the connection wraps in [`CliResponse::Item`] and POSTs
    /// back to the server as they arrive.
    type Stream: futures_util::Stream<
            Item = Result<crate::cli::command::ResponseItem, Self::Error>,
        > + Send
        + 'static;

    /// Run one command request, already deserialized off the wire by
    /// the connection.
    ///
    /// Returning `Err` means the run could not start (gate rejection,
    /// backend unavailable, …); item-level failures after a successful
    /// start ride the stream instead — and are NON-terminal on the
    /// wire. The stream ending is the end of the run — dropping the
    /// stream before it ends cancels the run.
    fn execute(
        &self,
        request: crate::cli::command::Request,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send;
}

/// One frame of the command-response leg — the body the connection
/// POSTs to `{mcp_url}/objectiveai/command`, one POST per frame, in
/// stream order. Every frame carries the request's server-minted
/// correlation `id` (from
/// [`CliRequestParams`](super::CliRequestParams)).
///
/// Frame rules — an exchange is `Ack (Item|Error)* Done`:
/// - [`CliResponse::Ack`] — ALWAYS the opening frame, POSTed the
///   moment the request is picked up, BEFORE the run starts: the
///   server learns a response is coming even when the run is slow to
///   produce its first item.
/// - [`CliResponse::Item`] — one typed command-output item.
/// - [`CliResponse::Error`] — a start failure or a stream error.
///   NON-terminal: a stream may yield errors and keep going.
/// - [`CliResponse::Done`] — ALWAYS the final frame of an exchange,
///   error or no. A run that fails to start still produces
///   `Ack`, `Error`, `Done`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "mcp.CliResponse")]
pub enum CliResponse {
    Ack { id: String },
    Item {
        id: String,
        item: crate::cli::command::ResponseItem,
    },
    Error { id: String, error: String },
    Done { id: String },
}

/// Error of [`NotSupportedMcpClientCommandExecutor`] — this client
/// does not execute CLI commands.
#[derive(Debug, thiserror::Error)]
#[error("CLI command execution is not supported by this MCP client")]
pub struct CommandExecutionNotSupported;

/// The [`McpClientCommandExecutor`] for clients that do NOT execute
/// commands — the default `E` of
/// [`Client`](super::Client) / [`Connection`](super::Connection).
///
/// A server that exposes the objectiveai capability may still send a
/// command request to such a client; `execute` answers with
/// [`CommandExecutionNotSupported`], which the connection's pump
/// reports to the server as a [`CliResponse::Error`] followed by the
/// terminal [`CliResponse::Done`].
#[derive(Debug, Clone, Copy, Default)]
pub struct NotSupportedMcpClientCommandExecutor;

impl McpClientCommandExecutor for NotSupportedMcpClientCommandExecutor {
    type Error = CommandExecutionNotSupported;
    type Stream = futures_util::stream::Empty<
        Result<crate::cli::command::ResponseItem, Self::Error>,
    >;

    async fn execute(
        &self,
        _request: crate::cli::command::Request,
    ) -> Result<Self::Stream, Self::Error> {
        Err(CommandExecutionNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_supported_executor_errors() {
        let executor = NotSupportedMcpClientCommandExecutor;
        let request = crate::cli::command::Request::Update(
            crate::cli::command::update::Request {
                path_type: crate::cli::command::update::Path::Update,
                base: crate::cli::command::RequestBase {
                    jq: None,
                    python: None,
                    timeout_seconds: None,
                    max_tokens: None,
                },
            },
        );
        let err = executor.execute(request).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "CLI command execution is not supported by this MCP client",
        );
    }

    #[test]
    fn cli_response_wire_shapes() {
        let ack = CliResponse::Ack {
            id: "7".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&ack).unwrap(),
            serde_json::json!({"type": "ack", "id": "7"}),
        );

        let error = CliResponse::Error {
            id: "7".to_string(),
            error: "boom".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "type": "error",
                "id": "7",
                "error": "boom",
            }),
        );

        let done = CliResponse::Done {
            id: "7".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&done).unwrap(),
            serde_json::json!({"type": "done", "id": "7"}),
        );

        // Item wraps a typed ResponseItem — spot-check the tag +
        // passthrough with a Python value item (the simplest leaf:
        // its ResponseItem variant is a bare JSON value).
        let item = CliResponse::Item {
            id: "7".to_string(),
            item: crate::cli::command::ResponseItem::Python(
                serde_json::json!({"ok": true}),
            ),
        };
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(value["type"], "item");
        assert_eq!(value["id"], "7");
        assert_eq!(value["item"], serde_json::json!({"ok": true}));

        // Round-trip through the wire form.
        let back: CliResponse =
            serde_json::from_str(&value.to_string()).unwrap();
        assert!(matches!(back, CliResponse::Item { id, .. } if id == "7"));
    }
}
