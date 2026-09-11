//! Opening the conduit.

use futures_util::StreamExt as _;

use super::{ExecuteError, ExecuteHandle, ExecuteStream};
use crate::server::container_client::ContainerClient;

/// Open `/postgres/{channel}` for the connection the container
/// announced.
///
/// # Why this one hands back two things
///
/// Because a connection is two directions and neither is the other's
/// subject. The [`ExecuteStream`] is everything the container wrote —
/// pgwire, client-first, so its startup message is the first thing
/// out — and the [`ExecuteHandle`] is how the database's side of the
/// conversation gets back to it. They are independent, and a tuple
/// rather than a type holding both, because a type holding both would
/// exist only to be taken apart.
///
/// # Declining
///
/// A server that has nothing to connect this to finishes the handle
/// at once: the clean close, before any byte, and the proxy shuts the
/// driver's socket — a server that hung up, which the driver already
/// knows how to report.
pub async fn execute(
    client: &ContainerClient,
    channel: u32,
) -> Result<(ExecuteStream, ExecuteHandle), ExecuteError> {
    let socket = client
        .open(&format!("/postgres/{channel}"))
        .await
        .map_err(ExecuteError::Open)?;
    let (sink, stream) = socket.split();
    Ok((ExecuteStream::new(stream), ExecuteHandle::new(sink)))
}
