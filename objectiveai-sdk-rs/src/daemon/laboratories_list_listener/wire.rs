//! Wire types for the daemon `/laboratories/list` endpoint.
//!
//! Each item is one laboratory's spec (the `create` echo shape) plus
//! its provenance (`source`, the unary `laboratories list` rules) and
//! a live `connected` flag from the daemon's `/laboratory` registry —
//! a field the unary command deliberately does not carry. Per-lab
//! attachment detail lives on the `/laboratories/{id}` endpoint's
//! [`super::super::laboratories_listener`] types.

use crate::laboratories::{EnvVar, Mount};

/// One laboratory on the `/laboratories/list` stream: its spec, the
/// machine whose host serves it, and whether that host is connected
/// right now. There is no local-vs-remote split — machine identity is
/// the only provenance.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "daemon.laboratories_list_listener.LaboratoryStatus")]
pub struct LaboratoryStatus {
    /// Raw (state-agnostic) laboratory id.
    pub id: String,
    pub image: crate::laboratories::LaboratoryImage,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    pub cwd: String,
    /// Unix seconds when the laboratory container was created, from
    /// podman's container record. `None` when the host didn't report
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// For agent laboratories: the full id of the agent the
    /// laboratory derives from. `None` for user-created laboratories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_full_id: Option<String>,
    /// The machine whose laboratory host serves this laboratory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<crate::machine::MachineIdentity>,
    /// The state (on that machine) the serving host serves —
    /// laboratory ids are only unique per (machine, state), so the
    /// stream may legitimately carry several same-id items that
    /// differ here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
    /// Whether the serving host's `/laboratory` connection is live
    /// right now.
    pub connected: bool,
    /// Whether the laboratory's CONTAINER is running right now. The
    /// lifecycle starts and stops containers on demand (MCP
    /// connections and filetree watchers are the demand), and every
    /// transition streams as an upsert — subscribers hold live state.
    /// Defaulted so frames predating this field parse (as
    /// not-running).
    #[serde(default)]
    pub running: bool,
}

/// One event on the `/laboratories/list` stream. The first is always
/// a [`Snapshot`](LaboratoryEvent::Snapshot); every later one upserts
/// or removes a single laboratory as the connected set or the local
/// scan changes.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "daemon.laboratories_list_listener.LaboratoryEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaboratoryEvent {
    /// The full laboratory set (connected ∪ local scan), sent once
    /// immediately on connect.
    #[schemars(title = "Snapshot")]
    Snapshot { laboratories: Vec<LaboratoryStatus> },
    /// A laboratory appeared or changed — connected, disconnected
    /// (but still locally present), entered the local scan, or had
    /// its identity re-announced. Full-value replace by `id`.
    #[schemars(title = "Upserted")]
    Upserted { laboratory: LaboratoryStatus },
    /// A laboratory vanished — deleted, or its serving host
    /// disconnected. Laboratory ids are only unique per (machine,
    /// state), so the pair disambiguates WHICH same-id laboratory
    /// left; an absent pair removes by bare id (legacy daemons).
    #[schemars(title = "Removed")]
    Removed {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine_state: Option<String>,
    },
}
