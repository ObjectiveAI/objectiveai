//! The refusals the handlers answer on their own, without asking the
//! manager: a name the caller has no image by, and a build that ended
//! without saying how.

use serde_json::json;

use crate::shared::error::Error;

/// The error for a name the caller has no image by: the manager
/// answered [`None`].
///
/// ```json
/// {"kind":"image","error":"no image named `tools`"}
/// ```
pub fn unknown(name: &str) -> Error {
    Error(json!({
        "kind": "image",
        "error": format!("no image named `{name}`"),
    }))
}

/// The error for a build whose stream ended without the image built,
/// the word that there was no room, or an error: the provider's build
/// said nothing of how it ended, and nothing is known to exist.
///
/// ```json
/// {"kind":"build","error":"the build ended without a result"}
/// ```
pub fn unfinished() -> Error {
    Error(json!({
        "kind": "build",
        "error": "the build ended without a result",
    }))
}
