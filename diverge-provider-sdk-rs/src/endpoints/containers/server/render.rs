//! The provider's own failures, in the wire's one error shape.

use std::fmt;

use crate::shared::error::Error;

/// Something between the provider and the proxy failed: a path that
/// would not open, a socket that died, an answer that would not
/// decode. The proxy is the provider's, so this is the provider's
/// failure, said plainly.
pub(crate) fn proxy(error: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "proxy",
        "error": error.to_string(),
    }))
}
/// A FUSE mount the proxy did not make, or whose fate was not heard.
pub(crate) fn mount_failed(id: &str, error: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "mount",
        "id": id,
        "error": error.to_string(),
    }))
}

/// A caller's content stopped before its finish.
pub(crate) fn content_stopped() -> Error {
    Error(serde_json::json!({
        "kind": "content",
        "error": "the caller's content stopped",
    }))
}
