//! The refusals the handlers answer on their own, without asking
//! the manager, and that no endpoint has a frame of its own for: a
//! name the caller has no volume by, a path that is not one of names,
//! and a write whose content did not arrive whole. A volume that
//! something holds is every endpoint's own variant — `Mounted`,
//! `FuseServed` and `Held` on each, and the run's
//! [`VolumeHeld`](crate::shared::containers::response::VolumeHeld),
//! [`VolumeMounted`](crate::shared::containers::response::VolumeMounted)
//! and
//! [`VolumeFuseServed`](crate::shared::containers::response::VolumeFuseServed)
//! — and no error here.

use serde_json::json;

use crate::shared::error::Error;

/// The error for a name the caller has no volume by:
/// `get`
/// answered [`None`].
///
/// ```json
/// {"kind":"volume","error":"no volume named `work`"}
/// ```
pub fn unknown(name: &str) -> Error {
    Error(json!({
        "kind": "volume",
        "error": format!("no volume named `{name}`"),
    }))
}

/// The error for a path with a component that is not a name — empty,
/// `.`, `..`, or holding a `/` or a NUL. What a read, a write and a
/// filetree answer before looking, since a provider descends names
/// and never resolves a path.
///
/// ```json
/// {"kind":"path","error":"`a/../b` is not a path of names"}
/// ```
pub fn path(components: &[String]) -> Error {
    Error(json!({
        "kind": "path",
        "error": format!("`{}` is not a path of names", components.join("/")),
    }))
}

/// The error for an empty path where a file is named: a read and a
/// write name a file, and the root is not one.
///
/// ```json
/// {"kind":"path","error":"the root is not a file"}
/// ```
pub fn root() -> Error {
    Error(json!({
        "kind": "path",
        "error": "the root is not a file",
    }))
}

/// The error for a write's content channel that ended without its
/// finish — the connection went first — so the content is not known
/// to be whole, and the write is abandoned.
///
/// ```json
/// {"kind":"content","error":"the content ended before its finish"}
/// ```
pub fn unfinished() -> Error {
    Error(json!({
        "kind": "content",
        "error": "the content ended before its finish",
    }))
}

/// The error for a frame on a write's content channel that will not
/// decode: the write is abandoned.
///
/// ```json
/// {"kind":"content","error":"a content frame did not decode: …"}
/// ```
pub fn content(error: &dyn std::fmt::Display) -> Error {
    Error(json!({
        "kind": "content",
        "error": format!("a content frame did not decode: {error}"),
    }))
}
