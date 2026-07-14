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
