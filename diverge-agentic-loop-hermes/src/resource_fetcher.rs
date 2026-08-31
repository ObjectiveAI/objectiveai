//! Asking for a resource and waiting out its delivery, as one call.

use std::error;
use std::fmt;
use std::string::FromUtf8Error;

use diverge_provider_sdk::agentic_loop_container::response::FetchResource;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::resource;

/// The round-trip a resource is: the ask onto the response stream,
/// the wait on the store's seal, the whole handed back — one
/// `fetch` per resource, however the deliveries interleave.
///
/// Built by the main endpoint, handed to the stream delegate: the
/// delegate calls [`fetch`](Self::fetch) for each `*_resource`
/// identity its agent carries, and the asks surface on the SSE
/// stream through the receiver [`new`](Self::new) returns beside
/// the fetcher.
///
/// # Parallel fetches are free
///
/// `fetch` takes `&self` and holds nothing across its wait: the ask
/// goes out on an unbounded sender (unbounded deliberately — asks
/// are tiny and bounded by the agent's resource fields, so there is
/// no backpressure story to buy), and the wait is the store's own
/// per-identity settlement. N concurrent fetches complete each as
/// its own resource settles, waiting on nothing else and
/// conflicting with nothing. Once per identity per run, though — collecting a
/// resource removes it — which is the natural shape: the run asks
/// once per resource field.
// The run that calls this is not implemented yet; the fetcher is
// the round-trip's client half, not dead weight.
#[allow(dead_code)]
pub struct ResourceFetcher {
    /// The ask half. Everything sent here must surface on the SSE
    /// stream as a `fetch_resource` event — the receiver rides with
    /// whoever builds that stream.
    asks: UnboundedSender<FetchResource>,
}

#[allow(dead_code)]
impl ResourceFetcher {
    /// The fetcher and the ask stream it feeds, a pair: the main
    /// endpoint constructs both, keeps the receiver for the
    /// response stream, and hands the fetcher to the delegate.
    pub fn new() -> (Self, UnboundedReceiver<FetchResource>) {
        let (asks, receiver) = tokio::sync::mpsc::unbounded_channel();
        (ResourceFetcher { asks }, receiver)
    }

    /// One resource, whole, as text.
    ///
    /// Sends the ask, waits for the delivery to settle — however
    /// long that takes; nothing in this protocol times anything out
    /// — and decodes the ASSEMBLED bytes as UTF-8: never the
    /// chunks, whose seams may split a multi-byte scalar. A
    /// delivery the server settled with its error route settles
    /// this call the same way.
    ///
    /// The `String` is this container's honest return, not the
    /// protocol's: a resource is arbitrary bytes everywhere else,
    /// and another container's fetcher may well hand back raw
    /// bytes. Hermes's resources are its state documents, which
    /// are text.
    pub async fn fetch(
        &self,
        identity: String,
    ) -> Result<String, FetchError> {
        let ask = FetchResource {
            r#type: Default::default(),
            identity: identity.clone(),
        };
        if self.asks.send(ask).is_err() {
            return Err(FetchError::Closed);
        }
        let bytes = resource::STORE
            .take(&identity)
            .await
            .map_err(FetchError::Failed)?;
        String::from_utf8(bytes).map_err(FetchError::Utf8)
    }
}

/// A fetch that cannot answer.
#[derive(Debug)]
pub enum FetchError {
    /// The ask channel is gone — the response stream died, so no
    /// delivery can come and waiting would be forever.
    Closed,
    /// The server failed the delivery — the bytes can never come
    /// (the client disconnected, holds nothing, …) — and this is
    /// its full error, verbatim off the error route.
    Failed(serde_json::Value),
    /// The settled bytes are not UTF-8, which this container's
    /// resources must be.
    Utf8(FromUtf8Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Closed => {
                f.write_str("the resource ask channel is closed")
            }
            FetchError::Failed(error) => {
                write!(f, "the server failed the resource: {error}")
            }
            FetchError::Utf8(error) => {
                write!(f, "the resource is not UTF-8: {error}")
            }
        }
    }
}

impl error::Error for FetchError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FetchError::Utf8(error) => Some(error),
            FetchError::Closed | FetchError::Failed(_) => None,
        }
    }
}
