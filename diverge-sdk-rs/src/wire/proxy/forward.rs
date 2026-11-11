//! One receiver's frames, delivered to the page, then the end.

use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::Semaphore;

use crate::wire::credit::Receiver;

/// Deliver every frame `receiver` yields, then the end.
///
/// The rules:
///
/// 1. Every frame is delivered as `Some(frame)`, whole, in the order
///    the receiver yielded it.
/// 2. When the receiver closes — the router forwarded a finish and
///    closed it, the scope ended, or the connection ended — `None`
///    is delivered exactly once, the end, and nothing after.
/// 3. After each frame, the next is taken off the receiver only once
///    the page says it took this one: a permit on `took`. Taking it
///    is what gives the far end room for another (see
///    [`credit`](crate::wire::credit)), so at most one frame waits
///    here and one in the page. `took` closed is the page gone.
/// 4. `deliver` answers whether the page is still listening. The
///    first `false` ends the forwarding at once: the receiver is
///    dropped, which frees the queue nobody reads and makes the
///    router's sends to it fail, which the router ignores. Nothing
///    further is delivered, the end included.
///
/// Why the receiver decides and not this function: the receiver
/// closes for exactly the reasons the page's own socket client
/// closes its receivers, so delivering its closure is delivering the
/// same fact the page would have had over a socket.
pub async fn forward<F>(mut receiver: Receiver, mut deliver: F, took: Arc<Semaphore>)
where
    F: FnMut(Option<Bytes>) -> bool,
{
    while let Some(frame) = receiver.recv().await {
        if !deliver(Some(frame)) {
            return;
        }
        // Rule 3.
        match took.acquire().await {
            Ok(permit) => permit.forget(),
            Err(_) => return,
        }
    }
    deliver(None);
}
