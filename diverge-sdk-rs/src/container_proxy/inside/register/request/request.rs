//! The arguments, the secrets and the template, as the request that
//! made the container carried them.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What the program is handed once: the `arguments` of the request
/// that made the container, typed to the same depth for the same
/// reason — a JSON value, because the image defines what it takes,
/// and what the value may be is what `GET /schema` answers — the
/// secrets the image needs, each by name to its value, and the id of
/// the template the container was made from, so that a program knows
/// which template it is an instance of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Request {
    /// The arguments, as the image defines them.
    pub arguments: Value,
    /// The secrets, each by the name the image declares it under in
    /// its template, to its value, any JSON: what the caller bound
    /// the name to, read when the container was made. In the order
    /// the template declares them. Absent, none.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub secrets: IndexMap<String, Value>,
    /// The template the container was made from, by id: an agent's
    /// agent template, a tool's tool template, or a dependency's
    /// dependency tool template — each the lowercase hexadecimal
    /// SHA-256 of the template's canonical bytes, as the caller that
    /// made the container names it.
    pub template: String,
}
