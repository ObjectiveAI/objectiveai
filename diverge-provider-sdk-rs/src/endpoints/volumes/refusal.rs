//! The two refusals the handlers answer on their own, without asking
//! the manager: a name the caller has no volume by, and a volume that
//! something is using.

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
/// mounted in a running container, or under a stat, an edit or a
/// delete in flight. The endpoints with a frame of their own for it
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
