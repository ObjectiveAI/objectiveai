//! Sending a frame, and saying a failure.

use std::fmt::Display;

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
