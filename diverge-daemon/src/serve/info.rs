//! Saying what the daemon is: its version.
//!
//! Served here, it needs no grant and reads nothing: every connection
//! the daemon admitted is answered, as the provider's `info` is. A
//! relayed one never reaches this — the dispatch hands it to the
//! [`relay`](super::relay) conduit first.

use diverge_sdk::daemon::endpoints::info::server::response::{Frame, Info};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::serve::reply;

/// The daemon's version: this crate's own, read from the manifest at
/// compile time. The daemon, the provider and the SDK are versioned
/// together, so it is also the revision of the protocol it speaks.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Answer the daemon's version and finish the scope.
pub async fn handle(scope: ScopeHandle) {
    reply::reply(&scope, &Frame::Info(Info { version: VERSION.to_owned() })).await;
    scope.send_response_finish().await;
}
