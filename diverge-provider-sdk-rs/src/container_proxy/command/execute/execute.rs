//! Opening a command.

use super::{ExecuteError, ExecuteHandle};
use crate::server::container_client::ContainerClient;

/// Open `/command/{channel}` for the ask on `channel`.
///
/// Nothing is sent here: what comes back is the handle to send on. A
/// channel the proxy does not know is `404`, one already being
/// answered `409`, both arriving as [`Open`](ExecuteError::Open).
pub async fn execute(
    client: &ContainerClient,
    channel: u32,
) -> Result<ExecuteHandle, ExecuteError> {
    let socket = client
        .open(&format!("/command/{channel}"))
        .await
        .map_err(ExecuteError::Open)?;
    Ok(ExecuteHandle::new(socket))
}
