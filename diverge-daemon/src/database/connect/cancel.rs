//! A cancel request, sent on for the container's own session.

use bytes::BytesMut;
use postgres_protocol::message::frontend;
use tokio::io::AsyncWriteExt as _;

use super::dial;
use crate::containers::Key;
use crate::daemon::Daemon;

/// Send the cancel on to the database, on a fresh connection as
/// Postgres expects, when the backend key is one a session of this
/// container was given; any other is dropped unanswered. A container
/// cancels only its own queries.
pub async fn cancel(daemon: &Daemon, key: Key, process_id: i32, secret_key: i32) {
    if daemon.live.backend_owner((process_id, secret_key)).await != Some(key) {
        return;
    }
    let Ok((mut server, _)) = dial::dial(&daemon.database).await else {
        return;
    };
    let mut request = BytesMut::new();
    frontend::cancel_request(process_id, secret_key, &mut request);
    let _ = server.write_all(&request).await;
    let _ = server.shutdown().await;
}
