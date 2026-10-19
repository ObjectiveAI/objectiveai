//! Stopping a program on the root.

use std::path::Path;

use diverge_sdk::file_lock;

use super::{Error, POLL, Program, http, port, say};

/// `program` on `root` stopped, and the command ended once the root's
/// lock is free.
///
/// - The root's lock is probed first — [`file_lock::try_exclusive`]
///   — and a lock nobody holds means no program is running on the
///   root: `<word> is not running` is printed and the command ends
///   well, since what was asked for is so.
/// - Else the program's port is found — see [`port`] — and
///   `POST /stop` is put to it over a loopback socket — see
///   [`http::stop`]. The server answers `204` and closes the
///   connection as it drains: the close is the accept, and nothing
///   more is asked of the socket. A program that is held by its lock
///   but not yet listening — still starting — refuses the connection,
///   and that is [`Error::NotListening`], reported rather than waited
///   for.
/// - Then, every 200 ms, the lock is probed until it is free. The
///   program lets its lock go last, after the whole of its teardown
///   — for the daemon, every run stopped, every dial ended and the
///   Postgres it started shut down; for the provider, every session
///   closed, every container and mount swept — so the lock free is
///   the completion, and `stopped <word>` is printed on it. The
///   program's own document is never consulted: the daemon stops the
///   Postgres it started, and the CLI need not know it had one.
pub async fn stop(root: &Path, program: Program) -> Result<(), Error> {
    let lock = program.lock(root);
    let probe = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)?;
    if probe.is_some() {
        drop(probe);
        return say(&format!("{} is not running", program.word())).await;
    }
    let port = port(root, program).await?;
    http::stop(port).await?;
    loop {
        let probe = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)?;
        if probe.is_some() {
            drop(probe);
            return say(&format!("stopped {}", program.word())).await;
        }
        tokio::time::sleep(POLL).await;
    }
}
