use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every JSON-RPC request the API forwards to the
/// client over the reverse-attach WS. Variant names follow the same
/// snake_case convention `client_request::Payload` uses; the
/// `serde(tag = "type")` discriminator pairs with
/// [`super::super::server_response::Payload`] by name.
///
/// MCP-routed variants carry `mcp_kind` directly on the variant
/// (alongside the typed params via `#[serde(flatten)]`). The non-MCP
/// `ReadMessageQueue` variant doesn't carry `mcp_kind` at all — it
/// hits the CLI's own local state and never routes to an upstream
/// MCP server. Use [`Payload::mcp_kind`] to retrieve it generically.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// POST `initialize`. The proxy's `protocolVersion` doesn't ride
    /// across this hop — the API discards it on the way in and
    /// substitutes its own `canonical_initialize_result` on the way
    /// out. Plugin arguments do NOT ride here — they are headers-only
    /// (see [`InitializeRequest`]).
    #[schemars(title = "Initialize")]
    Initialize {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: InitializeRequest,
    },

    /// POST `tools/list`.
    #[schemars(title = "ToolsList")]
    ToolsList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::tool::ListToolsRequest,
    },

    /// POST `tools/call`.
    #[schemars(title = "ToolsCall")]
    ToolsCall {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::tool::CallToolRequestParams,
    },

    /// POST `resources/list`.
    #[schemars(title = "ResourcesList")]
    ResourcesList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::resource::ListResourcesRequest,
    },

    /// POST `resources/read`.
    #[schemars(title = "ResourcesRead")]
    ResourcesRead {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::resource::ReadResourceRequestParams,
    },

    /// `DELETE` on the routed MCP URL — the proxy closing the
    /// session. No body beyond `mcp_kind`.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate { mcp_kind: super::super::McpKind },

    /// Read the CLI's local message queue for a given agent
    /// hierarchy. Non-MCP — no `mcp_kind`. Non-destructive: the
    /// API stamps the consumed ids onto the first
    /// `AssistantResponseChunk.request_message_ids` it emits so
    /// the downstream consumer owns row deletion; no separate
    /// release RPC.
    #[schemars(title = "ReadMessageQueue")]
    ReadMessageQueue(ReadMessageQueueRequest),

    /// Resolve a `Client` remote from the client's own local storage
    /// (agent / swarm / function / profile). Non-MCP — no `mcp_kind`.
    #[schemars(title = "Retrieve")]
    Retrieve(super::super::retrieve::Request),

    /// Run a SCRIPT agent's code on the client and return its output
    /// messages. Non-MCP — no `mcp_kind`: the CLI executes the code
    /// in-process on its embedded runtime (the same one the `python`
    /// command uses), with the FULL conversation as the script input.
    #[schemars(title = "Script")]
    Script(ScriptRequest),

    /// Forceful bulk teardown of every upstream connection for one
    /// objectiveai response id. Non-MCP — no `mcp_kind` (it spans all
    /// kinds for that response id). The CLI removes the whole
    /// response-id bucket from its connection registry, which drops
    /// every connection and kills every plugin subprocess under it.
    /// Distinct from `SessionTerminate` (graceful per-`mcp_kind` MCP
    /// `DELETE`); `Drop` is drop = kill.
    #[schemars(title = "Drop")]
    Drop(DropRequest),

    /// Begin streaming a tar export OUT of one laboratory on this
    /// conduit. Non-MCP — carries the laboratory id directly. The
    /// conduit opens the laboratory's `/export` and parks the byte
    /// stream under a fresh `transfer_id`; the requester then PULLS
    /// chunks with [`Payload::LaboratoryExportRead`]. Fully
    /// independent of any import — the splice (if any) happens on
    /// the requester's (API/proxy) side, so the peer laboratory may
    /// live on any host.
    #[schemars(title = "LaboratoryExportBegin")]
    LaboratoryExportBegin(LaboratoryExportBeginRequest),
    /// Pull the next chunk of a parked export stream. `eof: true` on
    /// the reply means the stream completed and the entry is gone
    /// (the final reply's `data` may still be non-empty).
    #[schemars(title = "LaboratoryExportRead")]
    LaboratoryExportRead(LaboratoryExportReadRequest),
    /// Drop a parked export stream early (requester-side failure).
    #[schemars(title = "LaboratoryExportAbort")]
    LaboratoryExportAbort(LaboratoryExportAbortRequest),
    /// Begin streaming a tar import INTO one laboratory on this
    /// conduit: the conduit opens the laboratory's `/import` with a
    /// channel-backed body and parks the sender under a fresh
    /// `transfer_id`; the requester then PUSHES chunks with
    /// [`Payload::LaboratoryImportWrite`] and closes with
    /// [`Payload::LaboratoryImportEnd`]. Independent of any export.
    #[schemars(title = "LaboratoryImportBegin")]
    LaboratoryImportBegin(LaboratoryImportBeginRequest),
    /// Push one chunk into a parked import body.
    #[schemars(title = "LaboratoryImportWrite")]
    LaboratoryImportWrite(LaboratoryImportWriteRequest),
    /// Close a parked import body and await the laboratory's unpack
    /// result. Replies with the total bytes fed.
    #[schemars(title = "LaboratoryImportEnd")]
    LaboratoryImportEnd(LaboratoryImportEndRequest),
    /// Drop a parked import early — the truncated tar makes the
    /// laboratory's unpack fail, so nothing partial is kept silently.
    #[schemars(title = "LaboratoryImportAbort")]
    LaboratoryImportAbort(LaboratoryImportAbortRequest),

    /// Transfer a path between TWO client laboratories on (possibly)
    /// different hosts: the CLI daemon drives the whole export→import
    /// splice itself with the half-op vocabulary above, holding at
    /// most one chunk in transit, and replies once with the byte
    /// total. Sent by the API's proxy when both endpoints are CLIENT
    /// laboratories that do NOT share a (machine, state) pair.
    #[schemars(title = "LaboratoryTransfer")]
    LaboratoryTransfer(LaboratoryTransferRequest),
    /// Transfer a path between two client laboratories that share the
    /// SAME (machine, state) — i.e. the same laboratory host. The CLI
    /// daemon forwards this verbatim to that host, which pipes the
    /// source's export stream straight into the destination's import
    /// (no chunk staging anywhere outside the host). Sent by the
    /// API's proxy when both endpoints are CLIENT laboratories with
    /// equal (machine, state) pairs.
    #[schemars(title = "LaboratoryLocalTransfer")]
    LaboratoryLocalTransfer(LaboratoryLocalTransferRequest),

    /// Execute a CLI command on the CLI daemon on behalf of a
    /// server-side PLUGIN (the command-execution extension: a plugin
    /// MCP server asked the proxy — its MCP client — to run a
    /// command). Non-MCP — no `mcp_kind`: the daemon executes the
    /// command itself in-process; `plugin` is scope identity, not
    /// routing.
    ///
    /// EVERY field is REQUIRED — no defaults, no header bags:
    /// - `identity`: the caller's agent identity, from the
    ///   proxy session's transient identity bag (the API stamps it on
    ///   every connect).
    /// - `plugin`: the four coordinates of the plugin whose MCP
    ///   server originated the request, from the typed
    ///   `X-MCP-Plugins` marker. The daemon stamps this trio on the
    ///   command's scope (the authenticated-conduit exception to
    ///   "never trust wire plugin identity") so the plugin run-gates
    ///   apply.
    /// - `request`: the typed CLI command to run.
    ///
    /// Answered by a MULTI-FRAME response: one
    /// [`super::super::server_response::Payload::Command`] frame per
    /// event, sharing this request's envelope id, streamed as items
    /// arrive — grammar `Ack (Item|Error)* Done`.
    #[schemars(title = "Command")]
    Command {
        identity: crate::identity::Identity,
        plugin: crate::mcp::server::Plugin,
        request: crate::cli::command::Request,
    },
}

impl Payload {
    /// Which CLI-hosted MCP server this payload targets. `Some` for
    /// the MCP-routed variants; `None` for `ReadMessageQueue` which
    /// hits the CLI's own local state.
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        match self {
            Payload::Initialize { mcp_kind, .. }
            | Payload::ToolsList { mcp_kind, .. }
            | Payload::ToolsCall { mcp_kind, .. }
            | Payload::ResourcesList { mcp_kind, .. }
            | Payload::ResourcesRead { mcp_kind, .. }
            | Payload::SessionTerminate { mcp_kind } => Some(mcp_kind.clone()),
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
            | Payload::Command { .. }
            => None,
        }
    }
}

/// Parameters for [`Payload::ReadMessageQueue`].
///
/// Two-rule predicate (now that PENDING is gone):
/// 1. Direct hit — `message_queue.agent_instance_hierarchy =
///    agent_instance_hierarchy`.
/// 2. BOUND-tag hit — `message_queue.agent_tag` resolves to a tag
///    whose `tags.agent_instance_hierarchy = agent_instance_hierarchy`.
///
/// The conduit-side spawn-with-tag flow pre-fires the tag-group
/// upgrade ahead of every read, so any tags sharing the spawn's
/// group become BOUND before the EXISTS-check runs and feed
/// straight into rule 2.
///
/// Returns rows oldest-first (`message_queue.id ASC`, which also
/// matches `message_queue.enqueued_at` ascending due to
/// AUTOINCREMENT). Row deletion is the downstream consumer's job:
/// the API stamps the consumed ids onto the first emitted
/// `AssistantResponseChunk.request_message_ids` so the consumer
/// knows which rows it owns.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.ReadMessageQueueRequest")]
pub struct ReadMessageQueueRequest {
    pub agent_instance_hierarchy: String,
}

/// Parameters for [`Payload::Drop`]. The objectiveai response id whose
/// entire upstream-connection bucket the CLI should tear down. The id
/// rides in the payload (not a header) because `Drop` is a control
/// message identified solely by its argument.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.DropRequest")]
pub struct DropRequest {
    pub response_id: String,
}

/// Parameters for [`Payload::LaboratoryExportBegin`]. `path` exports
/// cp-style (a directory is archived under its basename).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryExportBeginRequest")]
pub struct LaboratoryExportBeginRequest {
    pub laboratory_id: String,
    /// The exact laboratory host: machine id + its state. Laboratory
    /// ids are only unique per (machine, state); an absent pair falls
    /// back to first-match-by-id routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
    pub path: String,
}

/// Parameters for [`Payload::LaboratoryExportRead`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryExportReadRequest")]
pub struct LaboratoryExportReadRequest {
    pub transfer_id: String,
}

/// Parameters for [`Payload::LaboratoryExportAbort`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryExportAbortRequest")]
pub struct LaboratoryExportAbortRequest {
    pub transfer_id: String,
}

/// Parameters for [`Payload::LaboratoryImportBegin`]. The tar is
/// unpacked into `path` (created if missing).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryImportBeginRequest")]
pub struct LaboratoryImportBeginRequest {
    pub laboratory_id: String,
    /// The exact laboratory host: machine id + its state. Laboratory
    /// ids are only unique per (machine, state); an absent pair falls
    /// back to first-match-by-id routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
    pub path: String,
}

/// Parameters for [`Payload::LaboratoryImportWrite`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryImportWriteRequest")]
pub struct LaboratoryImportWriteRequest {
    pub transfer_id: String,
    /// RAW tar bytes. Never in the JSON header — rides OUT OF BAND in
    /// the binary wire frame (`crate::binary_frame`); serde skips it.
    #[serde(skip)]
    pub data: Vec<u8>,
}

/// Parameters for [`Payload::LaboratoryImportEnd`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryImportEndRequest")]
pub struct LaboratoryImportEndRequest {
    pub transfer_id: String,
}


/// Parameters for [`Payload::LaboratoryImportAbort`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryImportAbortRequest")]
pub struct LaboratoryImportAbortRequest {
    pub transfer_id: String,
}

/// Parameters for [`Payload::LaboratoryTransfer`] and
/// [`Payload::LaboratoryLocalTransfer`] — both endpoints' RAW ids and
/// exact host pairs (composite ids never ride the wire), plus the two
/// paths, `cp`-style.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryTransferRequest")]
pub struct LaboratoryTransferRequest {
    pub source_id: String,
    /// The source laboratory's exact host: machine id + its state.
    /// Absent pair (=) first-match-by-id routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source_machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source_machine_state: Option<String>,
    pub source_path: String,
    pub destination_id: String,
    /// The destination laboratory's exact host pair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub destination_machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub destination_machine_state: Option<String>,
    pub destination_path: String,
}

/// Parameters for [`Payload::LaboratoryLocalTransfer`] — identical
/// shape to [`LaboratoryTransferRequest`]; the (machine, state) pairs
/// are equal by construction (the proxy only picks the local variant
/// when they match).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryLocalTransferRequest")]
pub struct LaboratoryLocalTransferRequest {
    pub source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source_machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source_machine_state: Option<String>,
    pub source_path: String,
    pub destination_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub destination_machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub destination_machine_state: Option<String>,
    pub destination_path: String,
}

/// Parameters for [`Payload::Initialize`]. Currently EMPTY — kept as
/// a struct so the wire shape survives and future per-connect
/// parameters have a home.
///
/// Plugin arguments deliberately do NOT ride here: they are
/// HEADERS-ONLY — the API stamps the per-upstream
/// `X-OBJECTIVEAI-ARGUMENTS` header, which flows through this
/// request's envelope headers → the daemon's `/laboratory` forward
/// (`ChannelRequest.headers`) → the host's connect headers → the
/// plugin container's MCP server, on initialize and on every later
/// call.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.InitializeRequest")]
pub struct InitializeRequest {}

/// Parameters for [`Payload::Script`].
///
/// The identity fields mirror the MCP path's transient headers, but
/// TYPED (the non-MCP convention — cf. [`ReadMessageQueueRequest`]'s
/// `agent_instance_hierarchy`): the CLI applies them to the script's
/// execution context, so anything the script runs via
/// `objectiveai.execute` uses the calling agent's identity.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.ScriptRequest")]
pub struct ScriptRequest {
    /// The code to run — the agent's `type`-tagged script definition
    /// (`{"type":"python","python":"…"}`).
    pub script: crate::agent::script::Script,
    /// The FULL conversation the script receives as its input: the
    /// request messages plus everything the continuation carries, in
    /// order.
    pub messages: Vec<crate::agent::completions::message::Message>,
    /// Full slash-separated lineage of the agent running this script.
    pub agent_instance_hierarchy: String,
    /// Leaf agent id of the attempt running this script.
    pub agent_id: String,
    /// WF-level id: the primary agent's id concatenated with all
    /// fallback ids.
    pub agent_full_id: String,
    /// JSON-encoded `RemotePath` the agent was fetched from; `None`
    /// when the agent was supplied inline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<String>,
    /// The objectiveai response id of this agent run.
    pub response_id: String,
    /// The dash-joined sibling response-id group, when known. `None`
    /// ⇒ the execution context carries no group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_ids: Option<String>,
}

#[cfg(test)]
mod command_request_tests {
    use super::*;

    fn update_request() -> crate::cli::command::Request {
        crate::cli::command::Request::Update(
            crate::cli::command::update::Request {
                path_type: crate::cli::command::update::Path::Update,
                base: crate::cli::command::RequestBase {
                    jq: None,
                    python: None,
                    timeout_seconds: None,
                    max_tokens: None,
                },
            },
        )
    }

    /// The Command payload round-trips with every REQUIRED field, and
    /// carries no mcp_kind.
    #[test]
    fn command_round_trips_and_has_no_mcp_kind() {
        let payload = Payload::Command {
            identity: crate::identity::Identity {
                response_id: Some("resp-1".to_string()),
                ..Default::default()
            },
            plugin: crate::mcp::server::Plugin {
                owner: "acme".to_string(),
                name: "widgets".to_string(),
                version: "1.2.3".to_string(),
            },
            request: update_request(),
        };
        assert!(payload.mcp_kind().is_none());

        let envelope = super::super::Request {
            id: "cmd-1".to_string(),
            headers: indexmap::IndexMap::new(),
            payload,
        };
        let text = serde_json::to_string(&envelope).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["type"], "command");
        assert_eq!(value["plugin"]["owner"], "acme");
        assert_eq!(value["identity"]["response_id"], "resp-1");

        let back: super::super::Request = serde_json::from_str(&text).unwrap();
        assert!(matches!(
            back.payload,
            Payload::Command { plugin, .. } if plugin.name == "widgets",
        ));
    }

    /// Every Command field is REQUIRED — a frame missing `plugin` (or
    /// the others) does not parse.
    #[test]
    fn command_fields_are_required() {
        for missing in ["identity", "plugin", "request"] {
            let mut frame = serde_json::json!({
                "id": "cmd-2",
                "type": "command",
                "identity": {},
                "plugin": {
                    "owner": "a", "name": "b",
                    "version": "1.0.0", "mcp": "m",
                },
                "request": {"path_type": "update", "jq": null, "python": null},
            });
            frame.as_object_mut().unwrap().remove(missing);
            assert!(
                serde_json::from_str::<super::super::Request>(&frame.to_string())
                    .is_err(),
                "expected parse failure with {missing} absent",
            );
        }
    }
}
