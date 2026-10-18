//! The provider's own failures, in the wire's one error shape.

use std::fmt;

use diverge_sdk::shared::error::Error;

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

/// A mount path the request is refused for, and why.
pub(crate) fn path_refused(why: &str) -> Error {
    Error(serde_json::json!({
        "kind": "path",
        "error": why,
    }))
}

/// A FUSE id two mounts of the request share.
pub(crate) fn id_refused(id: &str) -> Error {
    Error(serde_json::json!({
        "kind": "id",
        "error": format!("two FUSE mounts with the id `{id}`"),
    }))
}

/// The caller did not answer the ask to register the container — its
/// dependencies deployed, its secrets' values given: a finish with
/// nothing before it, or a caller gone.
pub(crate) fn registration_unserved() -> Error {
    Error(serde_json::json!({
        "kind": "registration",
        "error": "the caller did not register the container",
    }))
}

/// The registration ask could not be written, or its answer could
/// not be read. The answer carries values, and `error` is the
/// decoder's word about its shape, never a value from it.
pub(crate) fn registration_failed(error: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "registration",
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
