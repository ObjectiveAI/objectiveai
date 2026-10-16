//! Sending a frame, and saying a failure.

use std::fmt::Display;

use diverge_sdk::shared::containers::fuse_mount;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Send `frame` as one response on the scope. A frame that will not
/// encode is the one failure with nowhere to go, and is not sent;
/// the scope's finish, which every handler sends last, is what the
/// client then reads.
pub async fn reply<F: Encode>(scope: &ScopeHandle, frame: &F) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}

/// The wire's error for a failure of the daemon's own: what every
/// endpoint's `Error` variant carries when the store could not be
/// read or written. One shape, `{"kind":"daemon","error":…}`, in the
/// daemon's own words.
pub fn failure(error: &impl Display) -> Error {
    Error(serde_json::json!({
        "kind": "daemon",
        "error": error.to_string(),
    }))
}

/// The wire's error for a request a provider's capabilities exclude,
/// which the daemon refused without asking the provider: one shape,
/// `{"kind":"unsupported","error":…}`, naming what the provider lacks.
pub fn unsupported(what: &str) -> Error {
    Error(serde_json::json!({
        "kind": "unsupported",
        "error": what,
    }))
}

/// The wire's error for a run whose image no provider can source,
/// which the daemon refused without asking any: one shape,
/// `{"kind":"no_source","error":…}`, saying why — no listed registry
/// a reference names, and no connected provider holding the image.
pub fn no_source(what: &str) -> Error {
    Error(serde_json::json!({
        "kind": "no_source",
        "error": what,
    }))
}

/// The wire's error for a patch the daemon could not apply, or whose
/// result is not what the record may be: one shape,
/// `{"kind":"patch","error":…}`, naming the operation and the place,
/// or the member the result got wrong. Nothing changed.
pub fn patch(what: &str) -> Error {
    Error(serde_json::json!({
        "kind": "patch",
        "error": what,
    }))
}

/// The wire's error for a FUSE mount the provider did not make into
/// a running container: one shape, `{"kind":"mount","refusal":…}`,
/// the provider's answer as it gave it — `"overlaps"`, `"taken"`,
/// `"unsupported"`, or `{"error":…}` in the proxy's words. The mount
/// is not made and not on record; what the provider mounted before
/// it is.
pub fn mount(refusal: &fuse_mount::response::Frame) -> Error {
    Error(serde_json::json!({
        "kind": "mount",
        "refusal": refusal,
    }))
}

