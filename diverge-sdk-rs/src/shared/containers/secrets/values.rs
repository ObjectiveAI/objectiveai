//! The secrets' values, by name.

use indexmap::IndexMap;
use serde_json::Value;

/// Secrets by name to value, any JSON: what the caller feeds a
/// container, and what the program hands back. Fed, it carries every
/// name the program declared, in the order declared — a `fixed` one
/// a value read from its key, a `rotating` one the value locked to
/// the container. Collected at an inactive, it carries every
/// `rotating` name the program declared, each with the value the
/// program leaves in it; a `fixed` name in it is ignored, and a
/// `rotating` name missing from it is the collect's error, never a
/// write-back. A value is never read by anything between the vault
/// and the program.
pub type Values = IndexMap<String, Value>;
