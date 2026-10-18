//! How much a caller may still use, asked of the provider.

use std::future::Future;

use diverge_sdk::provider::endpoints::capacity::server::response::Frame;

/// Answers one question: how much may this caller still use.
///
/// [`capacity`](diverge_sdk::provider::endpoints::capacity) is the
/// whole of what it serves: the four numbers of its [`Frame`] — the
/// room for a new volume, the room in the client image store, the
/// memory and the overlay disk a container run could still reserve —
/// each the most of its resource THIS caller could use up as of the
/// instant of the answer, as the provider allots it.
///
/// # Why it is told who is asking
///
/// Because the numbers may depend on it, and the wire says so: a
/// provider that partitions its callers answers the caller's share,
/// and a provider that lets every caller use everything it holds
/// answers what it holds. A quota on the caller is the provider's to
/// fold in; this crate folds in nothing and compares nothing.
///
/// # It reserves nothing
///
/// The numbers are facts about an instant. A request that relies on
/// one — a create, a build, a run — is judged on its own terms when
/// it arrives, and the number does not bind it.
///
/// # Every number is answered
///
/// Whatever the provider's capabilities hold out: a number for a
/// resource the provider does not keep is any number, which the wire
/// has the caller not read. `0` is the natural one.
pub trait CapacityReporter: Send + Sync {
    /// Whatever counting fails with. It has nowhere to go on the
    /// wire — the endpoint defines no failure — so the handler
    /// answers nothing, which the wire reads as a request not served.
    type Error;

    /// The four numbers, for `client_identity`, as of now.
    fn capacity(&self, client_identity: &str) -> impl Future<Output = Result<Frame, Self::Error>> + Send;
}
