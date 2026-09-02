//! What the fetcher asks for, as the driver sends it.

use diverge_provider_sdk::agentic_loop_container::response::FetchResource;

/// One ask, for the driver to put on the socket as the matching
/// frame: `Response::FetchResource` or `Response::FetchContinuation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// A resource, by identity.
    Resource(FetchResource),
    /// The continuation — the one thing a run resumes from, so
    /// nothing to name.
    Continuation,
}
