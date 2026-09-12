//! Taking `/requests`.

use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::ContainerClient;

/// Dial `/requests` and hear every ask the container makes.
///
/// One server at a time: the proxy refuses a second with `409`, which
/// arrives as [`Open`](ExecuteError::Open). The stream that comes back
/// yields each ask as it arrives and ends when the connection does —
/// a clean close from the proxy, or the socket going — and either way
/// every ask whose answer path has not opened is dead, per the wire.
/// Nothing is ever sent on this socket: dropping the stream closes it,
/// which is the server leaving.
pub async fn execute(client: &ContainerClient) -> Result<ExecuteStream, ExecuteError> {
    let socket = client.open("/requests").await.map_err(ExecuteError::Open)?;
    Ok(ExecuteStream::new(socket))
}
