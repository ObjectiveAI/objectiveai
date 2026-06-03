//! `InputValueLog` — on-disk shape of [`super::InputValue`].
//!
//! Mirrors `InputValue`'s recursive tree but every leaf becomes a
//! [`LogReference`] to its own file. Object/Array branches preserve
//! structure as `IndexMap<String, LogReference>` and
//! `Vec<LogReference>` respectively — each value/element references
//! a per-leaf file (which itself may be a sub-tree if the original
//! child was an Object or Array; in that case the file's content is
//! the serialized `InputValueLog` of that sub-tree).
//!
//! Per-leaf file naming: paths nest under
//! `<route_base>/input/<dotted-key-path>/<id>.<ext>` so the on-disk
//! layout mirrors the input tree shape directly.
//!
//! Three top-level variants, JSON-distinguishable when untagged:
//!
//! - `Reference` (object with `"type":"reference","path":"…"`) — the
//!   leaf was a primitive (`String`/`Integer`/`Number`/`Boolean`) or
//!   a `RichContentPart`. The referenced file contains the value
//!   itself (raw text for strings, JSON for everything else).
//! - `Object` (JSON map of refs) — the original was
//!   [`super::InputValue::Object`].
//! - `Array` (JSON list of refs) — the original was
//!   [`super::InputValue::Array`].
//!
//! Readers branching on JSON shape can pattern-match in that order
//! without ambiguity (LogReference has a known `"type"` discriminator;
//! Object and Array are map vs list at the JSON level).

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.InputValueLog")]
pub enum InputValueLog {
    #[schemars(title = "Reference")]
    Reference(LogReference),
    #[schemars(title = "Object")]
    Object(IndexMap<String, LogReference>),
    #[schemars(title = "Array")]
    Array(Vec<LogReference>),
}
