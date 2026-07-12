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
// Single-variant untagged enum: schemars emits a single-variant `anyOf` that
// the json-schema builder flattens to a bare top-level `$ref`, which the SDK
// code generators inline (breaking the schema roundtrip). The transform adds a
// sibling `"type": "object"` so it lands as the round-trippable `{type, $ref}`
// ref-wrapper shape. See `crate::json_schema::ref_wrapper_object`.
#[schemars(transform = crate::json_schema::ref_wrapper_object)]
pub enum Laboratory {
    /// A client-resolved laboratory, identified by an opaque `id`.
    Client(ClientLaboratory),
}

/// A client-resolved laboratory: a client-side MCP server keyed by an
/// opaque `id`. Wire shape: `{"type":"client","id":"…"}`.
///
/// Laboratory ids are only unique per (machine, state) — the same id
/// can exist on different laboratory hosts. `machine` + `machine_state`
/// pin THE laboratory this value means, so downstream routing (the
/// CLI conduit's `/laboratory` forward) is exact rather than
/// first-match-by-id. Absent pair ⇒ legacy id-only resolution.
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
    /// The machine id of the laboratory host serving this laboratory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine: Option<String>,
    /// The state (on that machine) the laboratory host serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub machine_state: Option<String>,
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
    // NOTE: no variant-level `#[schemars(title = "...")]`. This is a
    // single-variant unit enum, which schemars collapses to a bare string
    // schema and hoists the variant title to the schema's top-level
    // `title`. A title of "Client" makes the JS codegen route this type to
    // `src/client.ts` (clobbering the real SDK client). Leaving it off lets
    // the type's `rename` drive the title, matching its siblings.
    /// Serializes to `"client"`.
    Client,
}
