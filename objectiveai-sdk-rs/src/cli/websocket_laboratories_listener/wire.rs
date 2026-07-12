//! Wire types for the daemon `/laboratories/{id}` endpoint.
//!
//! One laboratory's full record: its spec (when present anywhere),
//! provenance + live connected-ness, and every attachment row
//! targeting it. Always sent as a full-value
//! [`LaboratoryInstanceEvent::Laboratory`] replace — the first frame
//! is the snapshot, every later frame supersedes it wholesale.

use crate::cli::command::laboratories::create::{EnvVar, Mount};
use crate::cli::command::laboratories::list::Source;

/// One attachment row targeting this laboratory: the agent target it
/// is attached to — an AIH or a tag, exactly one (the DB row's
/// CHECK-exclusive pair) — plus when and by whom.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_laboratories_listener.LaboratoryAttachment")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaboratoryAttachment {
    /// Attached directly to an agent instance hierarchy.
    #[schemars(title = "Aih")]
    Aih {
        agent_instance_hierarchy: String,
        /// Unix seconds when the attachment row was created.
        attached_at: i64,
        /// The AIH that performed the attach, when recorded.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        attached_by: Option<String>,
    },
    /// Attached to a tag (BOUND or GROUPED — the row targets the tag
    /// itself, wherever it points).
    #[schemars(title = "Tag")]
    Tag {
        tag: String,
        /// Unix seconds when the attachment row was created.
        attached_at: i64,
        /// The AIH that performed the attach, when recorded.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        attached_by: Option<String>,
    },
}

/// One laboratory's full record. Identity fields are present when the
/// laboratory is connected to the daemon or found by the local scan;
/// a laboratory known ONLY through attachment rows (or not at all)
/// zero-fills them — `source: None` marks "not present anywhere",
/// mirroring the agents `get_exact` zero-fill convention.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_laboratories_listener.LaboratoryRecord")]
pub struct LaboratoryRecord {
    /// Raw (state-agnostic) laboratory id.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub image: Option<String>,
    #[serde(default)]
    pub mounts: Vec<Mount>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cwd: Option<String>,
    /// Unix seconds when the laboratory container was created, from
    /// podman's container record. `None` when the identity source
    /// didn't report it (or the laboratory is known only through
    /// attachment rows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
    /// Where this laboratory lives relative to this machine + state —
    /// `None` when it is neither connected nor in the local scan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source: Option<Source>,
    /// Whether a live `/laboratory` manager connection for this id is
    /// registered with the daemon right now.
    pub connected: bool,
    /// Every attachment row targeting this laboratory, oldest first.
    #[serde(default)]
    pub attachments: Vec<LaboratoryAttachment>,
}

/// One event on the `/laboratories/{id}` stream: the record,
/// full-value. The first frame is the connect-time snapshot; every
/// later frame supersedes it wholesale.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_laboratories_listener.LaboratoryInstanceEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaboratoryInstanceEvent {
    /// The laboratory's current full record.
    ///
    /// NO variant-level `#[schemars(title)]` here, deliberately: a
    /// SINGLE-variant tagged enum collapses to the variant's inline
    /// schema, and a variant title would OVERWRITE the container's
    /// dot-path rename above — breaking the per-language codegen
    /// (module names key off the title). Restore per-variant titles
    /// only when a second variant joins.
    Laboratory { laboratory: LaboratoryRecord },
}
