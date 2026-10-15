//! Waiting for the port to accept.

use std::time::Duration;

use tokio::net::TcpStream;
use tokio::process::Child;

use super::Error;

/// How long the postmaster is given to accept on its port. Crash
/// recovery after an unclean stop can take a while.
const DEADLINE: Duration = Duration::from_secs(180);

/// How long one connection attempt is given.
const ATTEMPT: Duration = Duration::from_millis(250);

/// How long is waited between attempts.
const PAUSE: Duration = Duration::from_millis(100);

/// Wait until `127.0.0.1:<port>` accepts a connection, which is the
/// postmaster ready, or until `child` exits, which is the postmaster
/// failing — [`Error::Exited`] with how — or until the deadline,
/// [`Error::Unready`]. Accepting is all that is checked: nothing
/// speaks the protocol here.
pub async fn ready(child: &mut Child, port: u16) -> Result<(), Error> {
    let deadline = tokio::time::Instant::now() + DEADLINE;
    while tokio::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(Error::Wait)? {
            return Err(Error::Exited(status));
        }
        if let Ok(Ok(_)) = tokio::time::timeout(ATTEMPT, TcpStream::connect(("127.0.0.1", port))).await {
            return Ok(());
        }
        tokio::time::sleep(PAUSE).await;
    }
    Err(Error::Unready(port))
}
