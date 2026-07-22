//! The `/listen` broadcast's entire wire vocabulary — three shapes,
//! discriminated by `id` state plus the terminator's marker:
//!
//! - [`ListenerRequest<T>`] announces a run: the actual request under
//!   `value`, the broadcast stream `id`, and the producer's context.
//! - [`ListenerResponse<T>`] carries one response item: the bare
//!   `{id, value}` pair — no type tag; a consumer already knows how to
//!   deserialize each id's items from its opening request.
//! - [`ListenerEnd`] terminates a stream: `{id, end: true}`, exactly
//!   one per id when the producer's feed closes.
//!
//! The wrappers are generic on purpose: there are no per-leaf wrapper
//! types, and the generics don't ship in the json-schema set (the leaf
//! schemas cover the wire; `T` reads as the leaf `Request` /
//! `ResponseItem`, or `serde_json::Value` for generic consumption).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A run announcement on the `/listen` broadcast — see the module
/// docs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.broadcast_listener.ListenerRequest.{T}")]
pub struct ListenerRequest<T> {
    /// The broadcast stream id — fresh per run; every following frame
    /// for the run carries it.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_full_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_ids: Option<String>,
    /// The producer's PLUGIN CALLER identity (which installed plugin
    /// ran the command) — daemon-authored, absent for non-plugin
    /// producers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_version: Option<String>,
    /// The run was fired by the task scheduler — daemon-authored,
    /// `false` for every other producer. Always present; `default`
    /// only tolerates frames from older producers.
    #[serde(default)]
    pub task: bool,
    /// The run's actual request.
    pub value: T,
}

/// One response item on the `/listen` broadcast — see the module docs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.broadcast_listener.ListenerResponse.{T}")]
pub struct ListenerResponse<T> {
    /// The broadcast stream id of the run this item belongs to.
    pub id: String,
    /// One response item.
    pub value: T,
}

/// A stream terminator on the `/listen` broadcast — see the module
/// docs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.broadcast_listener.ListenerEnd")]
pub struct ListenerEnd {
    /// The broadcast stream id this terminator closes.
    pub id: String,
    /// Always `true` — the terminator marker.
    pub end: bool,
}
