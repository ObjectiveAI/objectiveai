//! The JSON-RPC request id.

/// A JSON-RPC request id — the spec restricts ids to strings and
/// numbers (fractional parts discouraged but representable;
/// [`serde_json::Number`] carries whatever the wire had, losslessly).
///
/// `Eq + Hash` so in-flight registries can key on the id directly
/// (`Number(7)` and `String("7")` are distinct keys, mirroring their
/// distinct wire forms `7` and `"7"`).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(untagged)]
#[schemars(rename = "mcp.RequestId")]
pub enum RequestId {
    Number(serde_json::Number),
    String(String),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{n}"),
            RequestId::String(s) => write!(f, "{s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Object / array / bool / null ids are rejected — the spec only
    /// permits strings and numbers.
    #[test]
    fn non_scalar_ids_rejected() {
        for raw in [
            serde_json::json!({"x": 1}),
            serde_json::json!([1]),
            serde_json::json!(true),
            serde_json::json!(null),
        ] {
            assert!(serde_json::from_value::<RequestId>(raw).is_err());
        }
    }
}
