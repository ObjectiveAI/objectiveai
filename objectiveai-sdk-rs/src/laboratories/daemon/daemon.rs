//! Laboratory-host channel envelopes.
//!
//! Every machine runs ONE resident `objectiveai-laboratory` HOST
//! process per state, serving ALL of that machine's laboratories. The
//! host dials OUT to each configured daemon's `/laboratory` WebSocket.
//! The wire there is:
//!
//! 1. **[`HostIdentify`]** — the FIRST text frame, before any
//!    authorization: the host's state, its [machine
//!    identity](crate::machine::MachineIdentity), and the FULL list of
//!    laboratories it serves (one [`Identify`] each).
//! 2. The standard `AuthEnvelope` (`{"signature": …}`) — authorization
//!    strictly FOLLOWS identity.
//! 3. Then a correlated request/response protocol: the daemon sends
//!    [`ChannelRequest`]s (the [`super::RequestPayload`]
//!    vocabulary, verbatim, with `laboratory_id` addressing the lab)
//!    and the host answers with [`ChannelResponse`]s — plus
//!    uncorrelated host→daemon [`HostNotification`]s whenever the
//!    host's laboratory set changes (create/delete), so every
//!    connected daemon's view stays current without scanning.
//! 4. The HOST-initiated lane: the host sends a
//!    [`HostCommandRequest`] (its OWN id space, host-minted) to run a
//!    CLI command on the daemon, answered by a MULTI-FRAME stream of
//!    [`HostCommandResponse`]s sharing that id — grammar
//!    `Ack (Item|Error)* Done`. The exact same exchange the
//!    API↔daemon reverse channel carries as its `Command` payloads,
//!    mirrored here because the two channels stay naive to each
//!    other's vocabulary (see [`super::RequestPayload`]'s module
//!    docs).
//!
//! The daemon reaches laboratories in-process: the conduit and the
//! `laboratories` commands call the resident laboratory registry
//! directly (`LaboratoryRegistry::forward` / `::list`), which forwards
//! over the owning host's `/laboratory` WS and correlates the reply.
//! There is no local-vs-remote split: whichever host serves the
//! laboratory serves the traffic, one code path, wherever it runs —
//! machine identity is the only provenance.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One bind mount in a laboratory's identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.IdentifyMount")]
pub struct IdentifyMount {
    pub host: String,
    pub container: String,
}

/// One laboratory's spec, as carried by [`HostIdentify`] and by
/// [`HostNotification::LaboratoryCreated`]. Mirrors the `laboratories
/// create` spec so `laboratories list` can echo it verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.Identify")]
pub struct Identify {
    /// The RAW, state-agnostic laboratory id — never prefixed or
    /// namespaced (the host's state scopes its container NAMEs, but
    /// the identity on this wire is the bare id).
    pub id: String,
    pub image: crate::laboratories::LaboratoryImage,
    pub mounts: Vec<IdentifyMount>,
    pub env: Vec<[String; 2]>,
    pub cwd: String,
    /// Unix seconds when the laboratory container was created, from
    /// podman's own container record. Optional + defaulted so frames
    /// from hosts predating this field still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// For agent laboratories: the full id of the agent the
    /// laboratory derives from. `None` for user-created laboratories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_full_id: Option<String>,
    /// For plugin laboratories: the plugin's canonical coordinate
    /// trio (owner/name lowercased, version verbatim — the repo's
    /// `v`-prefixed git tag). `None` for every other laboratory.
    /// Optional + defaulted so frames from hosts predating this field
    /// still parse (the `created_at` precedent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<IdentifyPlugin>,
    /// Whether the laboratory's container is RUNNING right now. The
    /// lifecycle starts and stops containers on demand, and the host
    /// re-announces on every transition
    /// ([`HostNotification::LaboratoryUpdated`]), so consumers hold
    /// live state. Defaulted so frames from hosts predating this
    /// field still parse (as not-running).
    #[serde(default)]
    pub running: bool,
}

/// A plugin laboratory's canonical coordinate trio, as carried by
/// [`Identify::plugin`]: owner/name lowercased, version verbatim (it
/// IS the repo's `v`-prefixed, case-sensitive git tag) — exactly the
/// identity the laboratory host derived the laboratory id and image
/// tag from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.IdentifyPlugin")]
pub struct IdentifyPlugin {
    pub owner: String,
    pub name: String,
    pub version: String,
}

/// The `/laboratory` connection's FIRST frame: who this HOST is. Sent
/// BEFORE the `AuthEnvelope` — identity always precedes authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.HostIdentify")]
pub struct HostIdentify {
    /// The state this host serves (its podman container names and
    /// locks are scoped to it). The daemon rejects hosts identifying a
    /// state other than its own.
    pub state: String,
    /// The machine this host runs on — the stable hashed id is the
    /// only provenance a laboratory has (there is no local-vs-remote).
    pub machine: crate::machine::MachineIdentity,
    /// EVERY laboratory this host serves right now, the full set. A
    /// reconnect re-sends the current set; later changes arrive as
    /// [`HostNotification`]s.
    pub laboratories: Vec<Identify>,
}

/// Daemon → host over the `/laboratory` WS: one correlated request.
/// `payload` is the reverse-attach vocabulary verbatim — the host is a
/// mini-conduit for all of its machine's laboratories.
///
/// `payload` is NESTED, never flattened: several payload variants
/// (e.g. `laboratory_create`) carry their own `id` field, which a
/// flatten would collide with the envelope's correlation `id` —
/// serialization emits the duplicate key, deserialization rejects it,
/// and the frame is silently dropped as forward-compat skip.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ChannelRequest")]
pub struct ChannelRequest {
    /// Correlation id, minted by the daemon; echoed by the response.
    pub id: String,
    /// The laboratory this request addresses. `None` only for ops that
    /// are host-level rather than lab-level. The daemon stamps it in
    /// `LaboratoryRegistry::forward`; the host demuxes on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratory_id: Option<String>,
    /// The originating request's headers (e.g.
    /// `X-OBJECTIVEAI-RESPONSE-ID`, which keys the host's per-session
    /// MCP connections).
    pub headers: IndexMap<String, String>,
    pub payload: super::RequestPayload,
}

/// Host → daemon: the reply to a [`ChannelRequest`], correlated by
/// `id`. `payload` nested for the same collision reason as
/// [`ChannelRequest`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ChannelResponse")]
pub struct ChannelResponse {
    pub id: String,
    pub payload: super::ResponsePayload,
}

/// Host → daemon, UNCORRELATED: the host's laboratory set (or a
/// served laboratory's live file tree) changed. The daemon's pump
/// tries [`ChannelResponse`] first (it has `id`), then this —
/// notifications never carry a correlation id. Sent to EVERY daemon
/// the host is connected to, so all views stay current.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.HostNotification")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostNotification {
    /// A laboratory was created on this host.
    #[schemars(title = "LaboratoryCreated")]
    LaboratoryCreated { laboratory: Identify },
    /// A laboratory's identity CHANGED — today that means its
    /// `running` state flipped (the lifecycle started or stopped its
    /// container). Daemons upsert exactly like `LaboratoryCreated`,
    /// so list subscribers hold live state.
    #[schemars(title = "LaboratoryUpdated")]
    LaboratoryUpdated { laboratory: Identify },
    /// A laboratory was deleted from this host.
    #[schemars(title = "LaboratoryDeleted")]
    LaboratoryDeleted { id: String },
    /// One live file-tree event from a laboratory this host watches:
    /// the host proxies the container MCP's `/filetree` SSE verbatim —
    /// every event it receives is forwarded here, unsolicited, to every
    /// connected daemon (which folds it into its own materialized tree
    /// and re-emits it on `/laboratories/{id}/filetree`). On attach the
    /// host sends a synthesized [`Snapshot`](crate::laboratories::filetree::FileTreeEvent::Snapshot)
    /// per watched laboratory, so a late-connecting daemon starts
    /// current — the same snapshot-then-deltas contract as the lab
    /// endpoint itself.
    #[schemars(title = "LaboratoryFiletree")]
    LaboratoryFiletree {
        id: String,
        event: crate::laboratories::filetree::FileTreeEvent,
    },
}

/// Host → daemon over the `/laboratory` WS: execute a CLI command on
/// the DAEMON on behalf of a plugin whose MCP server runs host-side —
/// the host-initiated twin of the reverse channel's `Command` payload,
/// working identically. Correlation `id` is HOST-minted and lives in
/// its own id space, unrelated to [`ChannelRequest`] ids (which the
/// daemon mints).
///
/// EVERY field is REQUIRED — no defaults, no header bags:
/// - `agent_arguments`: the calling agent's identity.
/// - `plugin`: the coordinates of the plugin whose MCP server
///   originated the command. The daemon stamps this trio on the
///   command's scope (this authenticated channel is, like the
///   conduit, a deliberate exception to "never trust wire plugin
///   identity") so the plugin run-gates apply.
/// - `request`: the typed CLI command to run.
///
/// Answered by a MULTI-FRAME reply: one [`HostCommandResponse`] per
/// event, sharing this request's `id`, streamed as items arrive —
/// grammar `Ack (Item|Error)* Done`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.HostCommandRequest")]
pub struct HostCommandRequest {
    /// Correlation id, minted by the HOST; echoed by every reply
    /// frame.
    pub id: String,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub plugin: crate::mcp::server::Plugin,
    pub request: crate::cli::command::Request,
}

/// Daemon → host: one frame of the MULTI-FRAME reply to a
/// [`HostCommandRequest`], correlated by `id` — the only exchange on
/// this wire where one request id is answered by many frames.
///
/// Wire: `{"id":…,"frame":"item","item":{…}}` — `frame` is
/// [`CommandFrame`]'s tag, flattened beside the id (no field
/// collision: the frame carries no `id` of its own).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.HostCommandResponse")]
pub struct HostCommandResponse {
    pub id: String,
    #[serde(flatten)]
    pub frame: CommandFrame,
}

/// One frame of a [`HostCommandRequest`] exchange. The grammar is
/// `Ack (Item|Error)* Done` — mirroring the reverse channel's
/// `client_objectiveai_mcp.server_response.CommandFrame` (a deliberate
/// LOCAL twin: the two channels never import each other's vocabulary):
///
/// - [`CommandFrame::Ack`] — ALWAYS the opening frame, sent the
///   moment the daemon picks the request up, BEFORE the run starts.
/// - [`CommandFrame::Item`] — one typed command-output item, sent AS
///   IT ARRIVES (never collected, never delayed).
/// - [`CommandFrame::Error`] — a start failure or a stream error.
///   NON-terminal: the stream may keep yielding after one.
/// - [`CommandFrame::Done`] — ALWAYS the final frame, error or no.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.CommandFrame")]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum CommandFrame {
    Ack,
    Item {
        item: crate::cli::command::ResponseItem,
    },
    Error { error: String },
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The daemon's inbound demux tries [`ChannelResponse`] first,
    /// then [`HostCommandRequest`], then [`HostNotification`] — a
    /// command request must never satisfy the earlier parses.
    #[test]
    fn host_command_request_is_not_a_channel_response() {
        let request = HostCommandRequest {
            id: "cmd-1".to_string(),
            agent_arguments: Default::default(),
            plugin: crate::mcp::server::Plugin {
                owner: "acme".to_string(),
                name: "widgets".to_string(),
                version: "1.2.3".to_string(),
                mcp: "main".to_string(),
            },
            request: crate::cli::command::Request::Update(
                crate::cli::command::update::Request {
                    path_type: crate::cli::command::update::Path::Update,
                    base: crate::cli::command::RequestBase {
                        jq: None,
                        python: None,
                        timeout_seconds: None,
                        max_tokens: None,
                    },
                },
            ),
        };
        let text = serde_json::to_string(&request).unwrap();
        assert!(serde_json::from_str::<ChannelResponse>(&text).is_err());
        let parsed: HostCommandRequest = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.id, "cmd-1");
        assert_eq!(parsed.plugin.owner, "acme");
    }

    /// Frame tags ride flattened beside the envelope id.
    #[test]
    fn host_command_response_wire_shape() {
        let ack = HostCommandResponse {
            id: "cmd-1".to_string(),
            frame: CommandFrame::Ack,
        };
        assert_eq!(
            serde_json::to_string(&ack).unwrap(),
            r#"{"id":"cmd-1","frame":"ack"}"#,
        );
        let error = HostCommandResponse {
            id: "cmd-1".to_string(),
            frame: CommandFrame::Error {
                error: "boom".to_string(),
            },
        };
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"id":"cmd-1","frame":"error","error":"boom"}"#,
        );
        let done: HostCommandResponse =
            serde_json::from_str(r#"{"id":"cmd-1","frame":"done"}"#).unwrap();
        assert!(matches!(done.frame, CommandFrame::Done));
    }
}
