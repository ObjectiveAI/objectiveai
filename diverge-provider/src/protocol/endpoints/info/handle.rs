//! Answering an info request, from a scope and the capabilities.

use diverge_sdk::provider::endpoints::info::server::response::{self, Capabilities};
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

/// Answer with the version and the capabilities, and end the scope.
///
/// One of the two handlers that ask a provider for a value and
/// nothing else — the other is
/// [`capacity`](crate::protocol::endpoints::capacity): here, the
/// [`Capabilities`] it holds out, which the dispatch was built with
/// and which every other handler enforces. The version is the
/// protocol's, compiled in.
///
/// # A provider that wants to answer another version does not call this
///
/// Which is the whole of the configurability of the version. A
/// provider speaking something other than what it linked has an
/// interesting problem, and a parameter here would be an invitation
/// to create one: the honest version is the one in the binary, and
/// anything else is a claim about code that is not running. The
/// capabilities are the provider's own claim, and the handlers hold
/// it to them.
///
/// # Nothing is checked, because everything was
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, so a scope arriving here already
/// parsed as an info ask — which carried nothing, the question
/// having no parameters.
pub async fn handle(scope: ScopeHandle, capabilities: &Capabilities) {
    let frame = response::Frame {
        version: VERSION.to_string(),
        capabilities: capabilities.clone(),
    };
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
