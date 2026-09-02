//! Asking the caller for things, and waiting out their delivery.
//!
//! A run needs two kinds of thing it does not have: the RESOURCES
//! its agent names by identity (OAuth state documents), and the
//! CONTINUATION it resumes from. Both are the caller's, both arrive
//! the same way — the container asks on its socket, the server
//! fetches from the caller and POSTs the bytes into a store on the
//! delivery routes, the store settles — and one [`Fetcher`] serves
//! both: one ask channel the driver drains onto the socket, one
//! method per kind, each awaiting its store.
//!
//! # What differs, and only that
//!
//! Where the bytes go. A resource is a small named document the
//! preparation consumes in memory, so [`store::resource`] keeps it
//! keyed by identity and [`fetch_resource`](Fetcher::fetch_resource)
//! hands back its text. The continuation is one large unnamed thing
//! that goes to disk as it lands, so [`store::continuation`] is a
//! single slot writing files, and
//! [`fetch_continuation`](Fetcher::fetch_continuation) hands back
//! only whether a session landed. Everything else — the ask, the
//! wait, the errors' shape — is the same.

mod ask;
mod continuation_error;
mod resource_error;

pub use ask::*;
pub use continuation_error::*;
pub use resource_error::*;

use diverge_provider_sdk::agentic_loop_container::response::FetchResource;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::store;

/// The asking half of the run: sends each ask through one channel
/// and waits on the store the delivery lands in.
///
/// Built by the main endpoint, which keeps the receiver for the
/// socket and hands the fetcher to the filesystem preparation. Every
/// method takes `&self` and holds nothing across its wait, so N
/// fetches run concurrently and conflict with nothing; the stores
/// judge repetition (a resource is collected once per identity, the
/// continuation once per run).
pub struct Fetcher {
    /// The ask half. Everything sent here must go out on the socket
    /// as the matching frame — the receiver rides with whoever
    /// drives that socket. Unbounded deliberately: asks are tiny and
    /// bounded by the agent's resource fields plus one.
    asks: UnboundedSender<Ask>,
}

impl Fetcher {
    /// The fetcher and the ask stream it feeds, a pair.
    pub fn new() -> (Self, UnboundedReceiver<Ask>) {
        let (asks, receiver) = tokio::sync::mpsc::unbounded_channel();
        (Fetcher { asks }, receiver)
    }

    /// One resource, whole, as text.
    ///
    /// Sends the ask, waits for the delivery to settle — however
    /// long that takes; nothing in this protocol times anything out
    /// — and decodes the ASSEMBLED bytes as UTF-8: never the chunks,
    /// whose seams may split a multi-byte scalar. A delivery the
    /// server settled with its error route settles this call the
    /// same way.
    ///
    /// The `String` is this container's honest return, not the
    /// protocol's: a resource is arbitrary bytes everywhere else.
    /// Hermes's resources are its state documents, which are text.
    pub async fn fetch_resource(
        &self,
        identity: String,
    ) -> Result<String, ResourceError> {
        let ask = FetchResource {
            identity: identity.clone(),
        };
        if self.asks.send(Ask::Resource(ask)).is_err() {
            return Err(ResourceError::Closed);
        }
        let bytes = store::resource::STORE
            .take(&identity)
            .await
            .map_err(ResourceError::Failed)?;
        String::from_utf8(bytes).map_err(ResourceError::Utf8)
    }

    /// The continuation, landed: `true` a session on disk to resume,
    /// `false` the fresh start.
    ///
    /// Sends the ask, waits for the delivery to settle — its chunks
    /// went to disk as they came — and for the delivered database to
    /// check out. Once per run: a second call is answered as the bug
    /// it is.
    pub async fn fetch_continuation(&self) -> Result<bool, ContinuationError> {
        if self.asks.send(Ask::Continuation).is_err() {
            return Err(ContinuationError::Closed);
        }
        store::continuation::STORE.take().await
    }
}
