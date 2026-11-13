//! A request's body as its command log keeps it: secrets replaced.

/// What a secret's value is replaced with.
pub const REDACTED: &str = "[redacted]";

/// The body of a request — the JSON past its tag — with every value a
/// secret could be in replaced.
///
/// The rules:
///
/// 1. `vault/insert`: `value` is the secret itself.
/// 2. `agents/exec`, `standalone_tools/exec`, `dependency_tools/exec`:
///    every value of `env`, which may be a secret; the names stay.
/// 3. `agents/frontend`, `standalone_tools/frontend`,
///    `dependency_tools/frontend`: every
///    header's value, which may be a secret — a cookie, a credential;
///    the names stay.
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
        // Rule 2.
        "agents/exec" | "standalone_tools/exec" | "dependency_tools/exec" => {
            if let Some(env) = body.get_mut("env").and_then(serde_json::Value::as_object_mut) {
                for value in env.values_mut() {
                    *value = serde_json::Value::String(REDACTED.to_string());
                }
            }
        }
        // Rule 3.
        "agents/frontend" | "standalone_tools/frontend" | "dependency_tools/frontend" => {
            if let Some(headers) = body.get_mut("headers").and_then(serde_json::Value::as_array_mut) {
                for header in headers {
                    if let Some(value) = header.as_array_mut().and_then(|pair| pair.get_mut(1)) {
                        *value = serde_json::Value::String(REDACTED.to_string());
                    }
                }
            }
        }
        _ => {}
    }
    body
}
