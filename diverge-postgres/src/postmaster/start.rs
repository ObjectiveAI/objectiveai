//! Spawning the postmaster on a free port.

use std::path::Path;
use std::process::Stdio;

use tokio::net::TcpListener;
use tokio::process::{Child, Command};

use super::Error;
use crate::install::Binaries;

/// The name every process of the postmaster carries, as its
/// `cluster_name`: `ps` shows `postgres: diverge-postgres: …`, which
/// is how a human finds what a start has not yet stopped.
pub const CLUSTER_NAME: &str = "diverge-postgres";

/// Start `postgres` on `data`, on a loopback port nobody holds, and
/// hand back the child and the port.
///
/// The port is one the kernel gave to a listener bound to
/// `127.0.0.1:0`, dropped the instant its number is read; the
/// postmaster binds it moments later, and a race for it in between
/// is the postmaster failing to start, which [`ready`](super::ready)
/// reports. The server listens on the loopback alone, with no Unix
/// socket, so there is nothing on disk to place or clean up; with
/// `max_connections` as configured; and under [`CLUSTER_NAME`]. Its
/// stdio is closed. It is not leashed: nothing ends it when this
/// program does.
pub async fn start(binaries: &Binaries, data: &Path, max_connections: u32) -> Result<(Child, u16), Error> {
    let port = free_port().await?;
    let mut postgres = Command::new(binaries.postgres());
    postgres
        .arg("-D")
        .arg(data)
        .arg("-p")
        .arg(port.to_string())
        .arg("-h")
        .arg("127.0.0.1")
        .arg("-c")
        .arg(format!("max_connections={max_connections}"))
        .arg("-c")
        .arg(format!("cluster_name={CLUSTER_NAME}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    #[cfg(unix)]
    postgres.arg("-c").arg("unix_socket_directories=");
    // CREATE_NO_WINDOW: no console window of its own.
    #[cfg(windows)]
    postgres.creation_flags(0x0800_0000);
    let child = postgres.spawn().map_err(|source| Error::Spawn {
        program: "postgres".to_string(),
        source,
    })?;
    Ok((child, port))
}

/// A loopback port nobody holds at this moment.
async fn free_port() -> Result<u16, Error> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(Error::Port)?;
    let port = listener.local_addr().map_err(Error::Port)?.port();
    drop(listener);
    Ok(port)
}
