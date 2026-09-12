//! Subscribing to the container's server.

use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::ContainerClient;

/// Open `/tool/notifications`; the stream is the subscription.
///
/// Nothing is sent. Dropping the stream closes the path, which is
/// the subscription ending; the container's server is not told and
/// need not be.
pub async fn execute(client: &ContainerClient) -> Result<ExecuteStream, ExecuteError> {
    let socket = client.open("/tool/notifications").await.map_err(ExecuteError::Open)?;
    Ok(ExecuteStream::new(socket))
}
