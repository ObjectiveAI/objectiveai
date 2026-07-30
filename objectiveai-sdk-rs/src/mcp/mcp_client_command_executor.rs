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
///
/// `Deserialize` is HAND-WRITTEN below, deliberately: serde's
/// internally-tagged derive buffers the whole frame through
/// `Content` before it can read `type`, and that buffering destroys
/// a [`RawValue`](serde_json::value::RawValue) — it travels as a
/// private token, exactly like the `arbitrary_precision` number
/// token, and neither survives being staged through serde's generic
/// value. Reading the map directly keeps `item` raw and the wire
/// byte-identical. Serialization is fine derived: field values go to
/// the real serializer, where `RawValue` emits its bytes verbatim.
#[derive(Debug, Clone, serde::Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "mcp.CliResponse")]
pub enum CliResponse {
    Ack { id: String },
    Item {
        id: String,
        /// One command-output item, kept as the producer's literal
        /// bytes and parsed exactly once, later, by whoever knows the
        /// type.
        ///
        /// Deliberately NOT the typed
        /// [`ResponseItem`](crate::cli::command::ResponseItem) sum.
        /// That sum is `#[serde(untagged)]` over every leaf in the
        /// command tree, so parsing an item into it picks whichever
        /// variant matches FIRST — and the receiver, which knows the
        /// one leaf type it asked for, then had to re-encode that
        /// guess to decode it properly. Any field the guessed variant
        /// did not model was gone by then: a `channels logs open`
        /// entry came back as "missing field `type`", because the
        /// variant that absorbed it did not carry the tag.
        ///
        /// Nor a [`serde_json::Value`], which would fix the routing
        /// but keep a lossy hop: a DOM normalizes numbers through
        /// `f64`/`u64` and re-serializes on the way out. The frame is
        /// a PIPE — its job is `id`, order and terminality, not
        /// understanding the payload — so it carries bytes, and the
        /// terminal decoder sees exactly what the producer wrote.
        ///
        /// The wire is UNCHANGED either way.
        #[schemars(with = "serde_json::Value")]
        item: Box<serde_json::value::RawValue>,
    },
    Error { id: String, error: String },
    Done { id: String },
}

impl<'de> serde::Deserialize<'de> for CliResponse {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};

        struct FrameVisitor;

        impl<'de> Visitor<'de> for FrameVisitor {
            type Value = CliResponse;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a cli response frame")
            }

            /// Read the map ONE FIELD AT A TIME, straight off the
            /// deserializer. `item` is pulled as a `RawValue` here,
            /// at the only moment its bytes are still available —
            /// serde's own tagged-enum path would have staged the
            /// whole frame through `Content` first and lost them.
            ///
            /// Fields are collected before dispatch because JSON
            /// object order is not guaranteed: `type` may arrive
            /// after the field it selects.
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<CliResponse, A::Error> {
                let mut kind: Option<String> = None;
                let mut id: Option<String> = None;
                let mut item: Option<Box<serde_json::value::RawValue>> = None;
                let mut error: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => kind = Some(map.next_value()?),
                        "id" => id = Some(map.next_value()?),
                        "item" => item = Some(map.next_value()?),
                        "error" => error = Some(map.next_value()?),
                        // Unknown keys are skipped, matching the
                        // derive's default tolerance.
                        _ => {
                            map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let kind = kind.ok_or_else(|| de::Error::missing_field("type"))?;
                let id = || id.ok_or_else(|| de::Error::missing_field("id"));
                match kind.as_str() {
                    "ack" => Ok(CliResponse::Ack { id: id()? }),
                    "item" => Ok(CliResponse::Item {
                        id: id()?,
                        item: item
                            .ok_or_else(|| de::Error::missing_field("item"))?,
                    }),
                    "error" => Ok(CliResponse::Error {
                        id: id()?,
                        error: error
                            .ok_or_else(|| de::Error::missing_field("error"))?,
                    }),
                    "done" => Ok(CliResponse::Done { id: id()? }),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["ack", "item", "error", "done"],
                    )),
                }
            }
        }

        deserializer.deserialize_map(FrameVisitor)
    }
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

        // Item carries the payload's raw bytes — spot-check the tag
        // and that the payload passes through untouched.
        let item = CliResponse::Item {
            id: "7".to_string(),
            item: serde_json::value::to_raw_value(
                &serde_json::json!({"ok": true}),
            )
            .unwrap(),
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
