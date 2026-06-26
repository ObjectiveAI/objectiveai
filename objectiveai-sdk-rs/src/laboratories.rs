//! Laboratories: completion-wide client-side MCP servers.
//!
//! A [`Laboratory`] attached to an agent completion is dialed by the proxy
//! as a client-side MCP upstream across *every* agent in the completion,
//! including fallbacks. Each laboratory is identified by an opaque `id`;
//! the proxy mirrors it as the URL `ws://laboratory/{id}` and the CLI conduit
//! routes it via the `id`-keyed [`crate::client_objectiveai_mcp::McpKind`]
//! variant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A laboratory attached to an agent completion — dialed by the proxy as a
/// client-side MCP upstream across every agent (and fallback).
///
/// Untagged: each variant's payload carries its own `type` discriminator
/// field, so the wire shape is exactly that of the inner struct (e.g.
/// `{"type":"client","id":"…"}`).
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "laboratories.Laboratory")]
pub enum Laboratory {
    /// A client-resolved laboratory, identified by an opaque `id`.
    Client(ClientLaboratory),
}

/// A client-resolved laboratory: a client-side MCP server keyed by an
/// opaque `id`. Wire shape: `{"type":"client","id":"…"}`.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "laboratories.ClientLaboratory")]
pub struct ClientLaboratory {
    /// Discriminator — always `"client"`.
    pub r#type: ClientLaboratoryType,
    /// The opaque laboratory id.
    pub id: String,
}

/// Discriminator for [`ClientLaboratory`]. Ser/de's to the static string
/// `"client"`.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "laboratories.ClientLaboratoryType")]
pub enum ClientLaboratoryType {
    /// Serializes to `"client"`.
    #[schemars(title = "Client")]
    Client,
}
