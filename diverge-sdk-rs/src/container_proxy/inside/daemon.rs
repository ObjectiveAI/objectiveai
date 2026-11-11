//! The daemon, reached through the proxy.


use crate::container_proxy::inside::{Client, Error, INSIDE_PORT};
use crate::wire::client::handle::Handle;
use crate::wire::connection::Connection;

/// The path the proxy serves the daemon protocol on.
const DAEMON: &str = "/daemon";

/// Where the program dials the daemon: the proxy, on the loopback.
pub fn daemon_url() -> String {
    format!("ws://127.0.0.1:{INSIDE_PORT}{DAEMON}")
}

impl Client {
    /// Dial the daemon through the proxy, and hold the connection.
    ///
    /// What comes back is a [`Handle`] of the daemon protocol's own
    /// wire, on which every endpoint's `client::execute` under
    /// [`daemon::endpoints`](crate::daemon::endpoints) runs as it
    /// would for any client: the program mints scopes and channels,
    /// the daemon answers, and the proxy carries every frame between
    /// them. No credential is presented and none is asked for — the
    /// connection is served under the permissions the tool's program
    /// declared in its [`register`](crate::container_proxy::inside::register) answer,
    /// and the proxy is the trust boundary — so the first frame on the socket is the
    /// first request. The router that reads the socket runs on a task
    /// of its own until the socket closes; the handle outlives nothing
    /// but it. A tool container's alone: an agent container asks the
    /// daemon nothing, and its proxy closes the
    /// socket at once.
    ///
    /// A program may dial more than once; each connection is its own,
    /// with scopes of its own.
    pub async fn daemon(&self) -> Result<Handle, Error> {
        let (socket, _) = tokio_tungstenite::connect_async(daemon_url())
            .await
            .map_err(Error::DaemonDial)?;
        let (handle, router) = Handle::open(Connection::Outgoing(socket));
        tokio::spawn(async move {
            let _ = router.run().await;
        });
        Ok(handle)
    }
}
