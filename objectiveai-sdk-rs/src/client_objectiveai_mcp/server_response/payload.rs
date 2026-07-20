use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every reply the CLI can send back to the API in
/// answer to a [`super::super::server_request::Payload`]. Variant
/// names pair 1:1 with the request side; the typed `result` /
/// `error` shape per JSON-RPC method is captured in
/// [`JsonRpcResult`].
///
/// MCP-routed variants echo `mcp_kind` on the variant itself
/// (lets the API sanity-check routing). Non-MCP variants
/// (`ReadMessageQueue` / `ClearMessageQueue`) don't carry
/// `mcp_kind` — they never had one to echo. Use
/// [`Payload::mcp_kind`] to retrieve it generically.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// Reply to
    /// [`super::super::server_request::Payload::Initialize`]. On
    /// success carries the upstream MCP session id the API stamps
    /// onto its outbound `Mcp-Session-Id` response header so the
    /// proxy adopts it. On failure (dial or aggregate-build error)
    /// carries a JSON-RPC error envelope the API translates into its
    /// own outbound error.
    #[schemars(title = "Initialize")]
    Initialize {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<InitializeReply>,
    },

    /// Reply to [`super::super::server_request::Payload::ToolsList`].
    #[schemars(title = "ToolsList")]
    ToolsList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::tool::ListToolsResult>,
    },

    /// Reply to [`super::super::server_request::Payload::ToolsCall`].
    #[schemars(title = "ToolsCall")]
    ToolsCall {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::tool::CallToolResult>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesList`].
    #[schemars(title = "ResourcesList")]
    ResourcesList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::resource::ListResourcesResult>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ResourcesRead`].
    #[schemars(title = "ResourcesRead")]
    ResourcesRead {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<crate::mcp::resource::ReadResourceResult>,
    },

    /// Acknowledges
    /// [`super::super::server_request::Payload::SessionTerminate`].
    /// On success carries the unit value (no body); on failure
    /// carries the upstream-delete error so the proxy sees a
    /// non-2xx and can retry.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        result: JsonRpcResult<()>,
    },

    /// Reply to
    /// [`super::super::server_request::Payload::ReadMessageQueue`].
    /// On success carries every matching queue row's id + body in
    /// oldest-first order; on failure surfaces the local storage
    /// error so the API can decide whether to retry. Non-MCP — no
    /// `mcp_kind` to echo.
    #[schemars(title = "ReadMessageQueue")]
    ReadMessageQueue(JsonRpcResult<ReadMessageQueueResult>),

    /// Reply to
    /// [`super::super::server_request::Payload::Retrieve`]. Carries the
    /// resolved definition (or `None` if not found) on success, or the
    /// client's local storage error. Non-MCP — no `mcp_kind`.
    #[schemars(title = "Retrieve")]
    Retrieve(JsonRpcResult<super::super::retrieve::Response>),

    /// Reply to
    /// [`super::super::server_request::Payload::Script`]. On success
    /// carries the script's output messages (assistant/tool only); on
    /// failure surfaces the execution error. Non-MCP — no `mcp_kind`.
    #[schemars(title = "Script")]
    Script(JsonRpcResult<ScriptResult>),

    /// Acknowledges
    /// [`super::super::server_request::Payload::Drop`]. Infallible — no
    /// `JsonRpcResult` wrapper; carries `dropped`: whether a bucket for
    /// the response id was present and removed. Non-MCP — no `mcp_kind`.
    #[schemars(title = "Drop")]
    Drop(DropResult),

    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryExportBegin`].
    /// Non-MCP — no `mcp_kind` (same for every transfer variant below).
    #[schemars(title = "LaboratoryExportBegin")]
    LaboratoryExportBegin(JsonRpcResult<LaboratoryTransferBeginResult>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryExportRead`].
    #[schemars(title = "LaboratoryExportRead")]
    LaboratoryExportRead(JsonRpcResult<LaboratoryExportChunk>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryExportAbort`].
    #[schemars(title = "LaboratoryExportAbort")]
    LaboratoryExportAbort(JsonRpcResult<LaboratoryTransferAck>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryImportBegin`].
    #[schemars(title = "LaboratoryImportBegin")]
    LaboratoryImportBegin(JsonRpcResult<LaboratoryTransferBeginResult>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryImportWrite`].
    #[schemars(title = "LaboratoryImportWrite")]
    LaboratoryImportWrite(JsonRpcResult<LaboratoryTransferAck>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryImportEnd`].
    /// On success carries the total bytes fed into the laboratory.
    #[schemars(title = "LaboratoryImportEnd")]
    LaboratoryImportEnd(JsonRpcResult<LaboratoryImportEndResult>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryImportAbort`].
    #[schemars(title = "LaboratoryImportAbort")]
    LaboratoryImportAbort(JsonRpcResult<LaboratoryTransferAck>),

    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryTransfer`] —
    /// the byte total the destination ingested.
    #[schemars(title = "LaboratoryTransfer")]
    LaboratoryTransfer(JsonRpcResult<LaboratoryTransferResult>),
    /// Reply to
    /// [`super::super::server_request::Payload::LaboratoryLocalTransfer`].
    #[schemars(title = "LaboratoryLocalTransfer")]
    LaboratoryLocalTransfer(JsonRpcResult<LaboratoryTransferResult>),

    /// One frame of the MULTI-FRAME reply to
    /// [`super::super::server_request::Payload::Command`] — the ONLY
    /// exchange on this wire where one request id is answered by many
    /// response frames, streamed as the command produces them.
    /// Non-MCP — no `mcp_kind`.
    ///
    /// Wire: `{"id":…,"type":"command","frame":"item","item":{…}}`.
    #[schemars(title = "Command")]
    Command {
        #[serde(flatten)]
        frame: CommandFrame,
    },
}

impl Payload {
    /// Which CLI-hosted MCP server produced this reply. `Some` for
    /// MCP-routed variants (echoes the request's `mcp_kind`); `None`
    /// for `ReadMessageQueue`.
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        match self {
            Payload::Initialize { mcp_kind, .. }
            | Payload::ToolsList { mcp_kind, .. }
            | Payload::ToolsCall { mcp_kind, .. }
            | Payload::ResourcesList { mcp_kind, .. }
            | Payload::ResourcesRead { mcp_kind, .. }
            | Payload::SessionTerminate { mcp_kind, .. } => Some(mcp_kind.clone()),
            Payload::ReadMessageQueue(_)
            | Payload::Retrieve(_)
            | Payload::Script(_)
            | Payload::Drop(_)
            | Payload::LaboratoryExportBegin(_)
            | Payload::LaboratoryExportRead(_)
            | Payload::LaboratoryExportAbort(_)
            | Payload::LaboratoryImportBegin(_)
            | Payload::LaboratoryImportWrite(_)
            | Payload::LaboratoryImportEnd(_)
            | Payload::LaboratoryImportAbort(_)
            | Payload::LaboratoryTransfer(_)
            | Payload::LaboratoryLocalTransfer(_)
            | Payload::Command { .. } => None,
        }
    }
}

/// One frame of a [`Payload::Command`] exchange. The exchange grammar
/// is `Ack (Item|Error)* Done` — mirroring the plugin-facing
/// `mcp.CliResponse` grammar:
///
/// - [`CommandFrame::Ack`] — ALWAYS the opening frame, sent the
///   moment the daemon picks the request up, BEFORE the run starts.
/// - [`CommandFrame::Item`] — one typed command-output item, sent AS
///   IT ARRIVES (never collected, never delayed).
/// - [`CommandFrame::Error`] — a start failure or a stream error.
///   NON-terminal: the stream may keep yielding after one.
/// - [`CommandFrame::Done`] — ALWAYS the final frame, error or no.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.CommandFrame")]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum CommandFrame {
    Ack,
    Item {
        item: crate::cli::command::ResponseItem,
    },
    Error { error: String },
    Done,
}

/// Successful payload for [`Payload::ReadMessageQueue`].
///
/// One [`ReadMessageQueueRow`] per consumed `message_queue` row,
/// in oldest-first id order. Each row's `content_ids` are the
/// `message_queue_contents.id`s for that row's slots; the row's
/// `rich_content` is the CLI's reconstructed payload for that
/// row alone (no cross-row separator splicing — callers join if
/// they want a unified User message).
///
/// Two consumers:
/// - **Startup snapshot** (`run_agent_loop`): joins every row's
///   parts with `"\n\n"` separators, flattens `content_ids`, and
///   stamps the result onto the first
///   `AssistantResponseChunk.request_message_ids`.
/// - **`ApiQueueDelegate`** (`agents logs read subscribe`-style
///   per-tool-response delivery): keeps rows separate so each
///   gets converted to its own `Vec<ContentBlock>` and surfaces
///   row-by-row on tool responses.
///
/// The downstream LogWriter resolves each id's kind at write
/// time (SQL CASE against `message_queue_contents.kind`) to
/// dispatch the right `logs.message_table` variant — kinds don't
/// need to ride on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.ReadMessageQueueResult")]
pub struct ReadMessageQueueResult {
    pub rows: Vec<ReadMessageQueueRow>,
}

/// Result of [`Payload::Drop`]. `dropped` is `true` if a connection
/// bucket for the response id was present and removed, `false` if no
/// bucket existed (the drop is idempotent either way).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.DropResult")]
pub struct DropResult {
    pub dropped: bool,
}

/// Successful payload for [`Payload::LaboratoryExportBegin`] /
/// [`Payload::LaboratoryImportBegin`] — the conduit-minted id the
/// requester uses for every subsequent op on this transfer half.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.LaboratoryTransferBeginResult")]
pub struct LaboratoryTransferBeginResult {
    pub transfer_id: String,
}

/// Successful payload for [`Payload::LaboratoryExportRead`] — one
/// pulled chunk. `eof: true` means the export completed and its entry
/// is gone (this final chunk's `data` may still be non-empty).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.LaboratoryExportChunk")]
pub struct LaboratoryExportChunk {
    /// Base64-encoded tar bytes.
    pub data: String,
    pub eof: bool,
}

/// Successful payload for the transfer acknowledgement replies
/// ([`Payload::LaboratoryExportAbort`], [`Payload::LaboratoryImportWrite`],
/// [`Payload::LaboratoryImportAbort`]) — no fields, the `Ok` is the ack.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.LaboratoryTransferAck")]
pub struct LaboratoryTransferAck {}

/// Successful payload for [`Payload::LaboratoryImportEnd`] — the total
/// archive bytes fed into the destination laboratory.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.LaboratoryImportEndResult")]
pub struct LaboratoryImportEndResult {
    pub bytes: u64,
}

/// Successful payload for [`Payload::LaboratoryTransfer`] /
/// [`Payload::LaboratoryLocalTransfer`]: the total bytes the
/// destination laboratory ingested.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.LaboratoryTransferResult")]
pub struct LaboratoryTransferResult {
    pub bytes: u64,
}

/// One queued row's payload + its content-slot ids.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.ReadMessageQueueRow")]
pub struct ReadMessageQueueRow {
    /// `message_queue_contents.id` of every slot in this row, in
    /// part order. Matches `rich_content`'s part count 1:1.
    pub content_ids: Vec<i64>,
    /// The row's content as the CLI reconstructed it.
    pub rich_content: crate::agent::completions::message::RichContent,
}

/// The successful `Initialize` payload — the upstream's verbatim
/// `InitializeResult` plus the native `Mcp-Session-Id` the CLI got
/// back from dialing the actual MCP server. The API forwards both
/// to the proxy: the result as the JSON-RPC body, the session id as
/// the `Mcp-Session-Id` response header. The CLI is a pure medium —
/// it doesn't synthesize capabilities, doesn't name itself, doesn't
/// pin a protocol version. Whatever the upstream MCP advertised is
/// what the proxy sees.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.InitializeReply")]
pub struct InitializeReply {
    /// Upstream's native `Mcp-Session-Id`. One per CLI-hosted MCP
    /// server — no aggregation, no encoding.
    pub mcp_session_id: String,
    /// The upstream's verbatim `InitializeResult` (capabilities,
    /// server info, protocol version). Returned as-is to the proxy.
    pub result: crate::mcp::initialize_result::InitializeResult,
}

/// JSON-RPC result/error shape for every typed method. Mirrors
/// the wire shape upstream MCP servers return (`{result: …}` on
/// success, `{error: {code, message, data?}}` on failure) but typed
/// at the SDK level instead of buried inside a `serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.JsonRpcResult.{R}", bound = "R: JsonSchema")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JsonRpcResult<R> {
    /// Method returned a typed result.
    #[schemars(title = "Ok")]
    Ok { result: R },
    /// Method returned a JSON-RPC error envelope.
    #[schemars(title = "Err")]
    Err {
        code: i64,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        data: Option<serde_json::Value>,
    },
}

/// Successful payload for [`Payload::Script`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.ScriptResult")]
pub struct ScriptResult {
    /// The script's output: the messages it appends to the
    /// conversation. Assistant/tool roles only — a script never puts
    /// words in the user's mouth.
    pub messages: Vec<crate::agent::script::OutputMessage>,
}

#[cfg(test)]
mod command_frame_tests {
    use super::*;

    #[test]
    fn command_frame_wire_shapes() {
        let ack = Payload::Command {
            frame: CommandFrame::Ack,
        };
        assert_eq!(
            serde_json::to_value(&ack).unwrap(),
            serde_json::json!({"type": "command", "frame": "ack"}),
        );

        let error = Payload::Command {
            frame: CommandFrame::Error {
                error: "boom".to_string(),
            },
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "type": "command",
                "frame": "error",
                "error": "boom",
            }),
        );

        let done = Payload::Command {
            frame: CommandFrame::Done,
        };
        assert_eq!(
            serde_json::to_value(&done).unwrap(),
            serde_json::json!({"type": "command", "frame": "done"}),
        );

        // Item wraps a typed ResponseItem — the Python variant is a
        // bare JSON value (untagged), the simplest leaf.
        let item = Payload::Command {
            frame: CommandFrame::Item {
                item: crate::cli::command::ResponseItem::Python(
                    serde_json::json!({"ok": true}),
                ),
            },
        };
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "command",
                "frame": "item",
                "item": {"ok": true},
            }),
        );

        // Round-trip through the wire form (full Response envelope).
        let envelope = super::super::Response {
            id: "req-1".to_string(),
            payload: item,
        };
        let text = serde_json::to_string(&envelope).unwrap();
        let back: super::super::Response = serde_json::from_str(&text).unwrap();
        assert_eq!(back.id, "req-1");
        assert!(matches!(
            back.payload,
            Payload::Command {
                frame: CommandFrame::Item { .. },
            },
        ));
    }
}
