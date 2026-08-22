//! Answering a version request, from a scope and nothing else.

use super::super::response;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::version::client::request;
use crate::server::scope_handle::ScopeHandle;

/// What this crate's version is.
///
/// Taken from the manifest at compile time, so it is the version of the
/// specification the binary was BUILT against rather than one written
/// down twice and left to drift. A crate that answers with this is
/// answering with what it actually implements.
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
/// # A request that will not decode is answered with silence
///
/// There is nowhere else for it to go. This endpoint's response is a
/// string and has no error case — deliberately, since the question has
/// no parameters and so nothing to get wrong — so the scope finishes
/// without a response and a caller reads that as
/// [`Unanswered`](crate::endpoints::version::client::execute::ExecuteError::Unanswered).
///
/// It is checked at all because this is public and a scope is a scope.
/// Whatever dispatches here read the tag to get here; this confirms it
/// rather than trusting it.
pub async fn handle(scope: ScopeHandle) {
    if request::Frame::decode(scope.request()).is_ok() {
        let mut buffer = Vec::new();
        response::Frame(VERSION)
            .encode(&mut Writer::new(&mut buffer))
            // The response is a string written as bytes. There is no
            // failure to handle, and the compiler agrees.
            .unwrap_or_else(|error| match error {});
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
