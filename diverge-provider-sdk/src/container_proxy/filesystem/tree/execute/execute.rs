//! Opening the watch.

use super::{ExecuteError, ExecuteStream};
use crate::server::container_client::ContainerClient;

/// Open `/filesystem/tree`: a fresh subscription, starting whole.
///
/// Nothing is sent. The stream yields the snapshot and then every
/// change for as long as it is held; dropping it closes the socket,
/// which the proxy treats as the server leaving, and a second call is
/// a second watch. A watch the proxy could not begin arrives as the
/// stream's one error.
pub async fn execute(client: &ContainerClient) -> Result<ExecuteStream, ExecuteError> {
    let socket = client.open("/filesystem/tree").await.map_err(ExecuteError::Open)?;
    Ok(ExecuteStream::new(socket))
}
