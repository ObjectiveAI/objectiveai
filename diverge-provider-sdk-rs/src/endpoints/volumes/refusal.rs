//! The refusals the handlers answer on their own, without asking
//! the manager: a name the caller has no volume by, a volume that
//! something is using, a path that is not one of names, and a write
//! whose content did not arrive whole.

use serde_json::json;

use crate::shared::error::Error;

/// The error for a name the caller has no volume by:
/// [`get`](crate::server::volume_manager::VolumeManager::get)
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

/// The error for a volume whose exclusive hold,
/// [`lock`](crate::server::volume::Volume::lock), could not be taken:
/// mounted in a running container or served, or under a stat, a
/// read, a write, a filetree, an edit or a delete in flight. What a
/// stat, a read, a write, a filetree and an edit answer, since none
/// has a frame of its own for it — and what a serve answers for a
/// volume held exclusively, or a persistent one held at all, its
/// hold being the shared one. The endpoints with a frame of their own for it
/// — a delete's
/// [`Mounted`](crate::endpoints::volumes::delete::server::response::Frame::Mounted),
/// a run's
/// [`VolumeHeld`](crate::shared::containers::response::VolumeHeld)
/// — answer with that instead.
///
/// ```json
/// {"kind":"mounted","error":"the volume `work` is mounted in a running container"}
/// ```
pub fn mounted(name: &str) -> Error {
    Error(json!({
        "kind": "mounted",
        "error": format!("the volume `{name}` is mounted in a running container"),
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
