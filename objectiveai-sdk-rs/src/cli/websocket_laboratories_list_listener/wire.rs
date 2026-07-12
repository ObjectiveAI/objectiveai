//! Wire types for the daemon `/laboratories/list` endpoint.
//!
//! Each item is one laboratory's spec (the `create` echo shape) plus
//! its provenance (`source`, the unary `laboratories list` rules) and
//! a live `connected` flag from the daemon's `/laboratory` registry —
//! a field the unary command deliberately does not carry. Per-lab
//! attachment detail lives on the `/laboratories/{id}` endpoint's
//! [`super::super::websocket_laboratories_listener`] types.

use crate::cli::command::laboratories::create::{EnvVar, Mount};
use crate::cli::command::laboratories::list::Source;

/// One laboratory on the `/laboratories/list` stream: its spec, where
/// it lives relative to this machine + state, and whether it is
/// connected to the daemon right now.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_laboratories_list_listener.LaboratoryStatus")]
pub struct LaboratoryStatus {
    /// Raw (state-agnostic) laboratory id.
    pub id: String,
    pub image: String,
    pub mounts: Vec<Mount>,
    pub env: Vec<EnvVar>,
    pub cwd: String,
    /// Unix seconds when the laboratory container was created, from
    /// podman's container record. `None` when the manager/scan didn't
    /// report it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// Where this laboratory lives relative to this machine + state —
    /// the same RAW-id classification as the unary `laboratories
    /// list`.
    pub source: Source,
    /// Whether a live `/laboratory` manager connection for this id is
    /// registered with the daemon right now.
    pub connected: bool,
}

/// One event on the `/laboratories/list` stream. The first is always
/// a [`Snapshot`](LaboratoryEvent::Snapshot); every later one upserts
/// or removes a single laboratory as the connected set or the local
/// scan changes.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_laboratories_list_listener.LaboratoryEvent")]
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
    /// A laboratory vanished from BOTH halves — no live connection
    /// and absent from the local scan.
    #[schemars(title = "Removed")]
    Removed { id: String },
}
