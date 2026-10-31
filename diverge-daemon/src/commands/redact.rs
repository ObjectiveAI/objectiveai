//! A request's body as its command log keeps it: secrets replaced.

/// What a secret's value is replaced with.
pub const REDACTED: &str = "[redacted]";

/// The body of a request — the JSON past its tag — with every value a
/// secret could be in replaced.
///
/// The rules:
///
/// 1. `vault/insert`: `value` is the secret itself.
///
/// A body that is not JSON is kept as `null`. A request that carries a
/// secret value adds its rule here.
pub fn body(path: &str, payload: &[u8]) -> serde_json::Value {
    let Some(json) = payload.get(1..) else {
        return serde_json::Value::Null;
    };
    let mut body: serde_json::Value = serde_json::from_slice(json).unwrap_or(serde_json::Value::Null);
    match path {
        // Rule 1.
        "vault/insert" => {
            if let Some(object) = body.as_object_mut()
                && object.contains_key("value")
            {
                object.insert("value".to_string(), serde_json::Value::String(REDACTED.to_string()));
            }
        }
        _ => {}
    }
    body
}
