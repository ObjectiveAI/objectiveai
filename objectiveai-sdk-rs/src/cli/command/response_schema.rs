//! [`ResponseSchema`] — newtype wrapper around [`schemars::Schema`]
//! that delegates its `JsonSchema` impl to [`serde_json::Value`].
//!
//! Every cli `*-schema` leaf subcommand emits a JSON Schema document
//! as its response. The wire shape is opaque JSON — any valid schema
//! is acceptable. Typing the response as `schemars::Schema` directly
//! makes that wire shape strongly typed, but `schemars::Schema`'s own
//! JsonSchema impl emits `{"type": ["object", "boolean"]}` (because
//! the JSON Schema spec says a (sub)schema can be either an object or
//! a bare boolean). That type array trips the
//! `no_type_arrays_outside_properties` builder property test.
//!
//! The orphan rule prevents us from `impl JsonSchema for schemars::Schema`
//! directly. This wrapper provides the same in-process typing
//! (a `schemars::Schema` inside) while overriding the JsonSchema impl
//! to behave exactly like `serde_json::Value` (no constraints, no
//! type array). Serialization is transparent — the wire shape is
//! identical to `schemars::Schema`'s.

use objectiveai_sdk_macros::schema_override;

#[schema_override(Ref)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ResponseSchema(pub schemars::Schema);

impl schemars::JsonSchema for ResponseSchema {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        <serde_json::Value as schemars::JsonSchema>::schema_name()
    }

    fn json_schema(
        generator: &mut schemars::SchemaGenerator,
    ) -> schemars::Schema {
        <serde_json::Value as schemars::JsonSchema>::json_schema(generator)
    }
}
