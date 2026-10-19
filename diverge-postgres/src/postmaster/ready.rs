//! Waiting for the postmaster to accept connections.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::{Child, Command};

use super::Error;
use crate::install::Binaries;

/// How long the postmaster is given to accept. Crash recovery after
/// an unclean stop can take a while.
const DEADLINE: Duration = Duration::from_secs(180);

/// How long is waited between asks.
const PAUSE: Duration = Duration::from_millis(100);

/// `pg_isready`'s exit code for a server that accepts connections.
const ACCEPTING: i32 = 0;

/// Wait until the postmaster on `127.0.0.1:<port>` accepts
/// connections — `pg_isready` answering `0` — or until `child`
/// exits, which is the postmaster failing — [`Error::Exited`] with
/// how — or until the deadline, [`Error::Unready`]. The ask is the
/// protocol's own, not the socket's: a postmaster accepts the socket
/// while it is still starting up and refuses the connection with
/// "the database system is starting up", and `pg_isready` answers
/// `1` to that, so a Ready announced here is one the first store
/// connection can rely on. A `pg_isready` that cannot be run at all
/// is [`Error::Probe`].
pub async fn ready(binaries: &Binaries, child: &mut Child, port: u16) -> Result<(), Error> {
    let deadline = tokio::time::Instant::now() + DEADLINE;
    while tokio::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(Error::Wait)? {
            return Err(Error::Exited(status));
        }
        let status = Command::new(binaries.pg_isready())
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(Error::Probe)?;
        if status.code() == Some(ACCEPTING) {
            return Ok(());
        }
        tokio::time::sleep(PAUSE).await;
    }
    Err(Error::Unready(port))
}
