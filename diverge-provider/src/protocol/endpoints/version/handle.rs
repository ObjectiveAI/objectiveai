//! Answering a version request, from a scope and nothing else.

use diverge_sdk::provider::endpoints::version::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// What this crate's version is: the specification's revision.
///
/// Taken from the manifest at compile time. This crate is versioned
/// with the SDK, which is the specification, by the one release step
/// that bumps the workspace, so what is answered is the revision the
/// binary was BUILT against rather than one written down twice and
/// left to drift. A crate that answers with this is answering with
/// what it actually implements.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Answer with the version and end the scope.
///
/// The one handler that asks a provider for nothing. Every other takes
/// something a provider supplied — a manager, a deployer, an identity —
/// because every other question is about that provider's own state.
/// This one is about the protocol, and the protocol is compiled in.
///
/// # A provider that wants to answer differently does not call this
///
/// Which is the whole of the configurability here. A provider speaking
/// something other than what it linked has an interesting problem, and
/// a parameter on this function would be an invitation to create one:
/// the honest version is the one in the binary, and anything else is a
/// claim about code that is not running.
///
/// # Nothing is checked, because everything was
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, so a scope arriving here already
/// parsed as a version ask — which carried nothing, the question
/// having no parameters. There is nothing left to look at, and this
/// takes the scope alone.
pub async fn handle(scope: ScopeHandle) {
    let mut buffer = Vec::new();
    response::Frame(VERSION)
        .encode(&mut Writer::new(&mut buffer))
        // The response is a string written as bytes. There is no
        // failure to handle, and the compiler agrees.
        .unwrap_or_else(|error| match error {});
    scope.send_response(&buffer).await;
    scope.send_response_finish().await;
}
