//! The supervisor's life: start, announce, wait, stop.

use std::path::PathBuf;
use std::pin::pin;

use diverge_sdk::config::daemon::Postgres;
use diverge_sdk::file_lock;
use diverge_sdk::postgres_supervisor::{Command, Ready};
use futures_util::future::{self, Either};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use super::Error;
use crate::cluster;
use crate::install;
use crate::postmaster;

/// Run the cluster the daemon's `postgres` asks for, under
/// `<root>/daemon/postgres/`, until a shutdown line, a signal, or the
/// postmaster's own exit. The remote kind asks for none, and is
/// [`Error::Remote`].
///
/// In order: the directory made, [`install::ensure`], the init lock, [`cluster::password`],
/// [`cluster::init`], [`postmaster::stop`] of whatever the last start
/// left, [`postmaster::start`], [`postmaster::ready`] — a postmaster
/// that does not become ready is stopped again before the error is
/// returned — the lock let go, the [`Ready`] line on stdout, and the
/// wait. A shutdown line or a signal stops the postmaster fast and is
/// `Ok`; the postmaster exiting on its own is [`Error::Exited`].
pub async fn run(postgres: Postgres, root: PathBuf) -> Result<(), Error> {
    let max_connections = match postgres {
        Postgres::Local { max_connections } => max_connections,
        Postgres::Remote { .. } => return Err(Error::Remote),
    };
    let dir = root.join("daemon").join("postgres");
    tokio::fs::create_dir_all(&dir).await.map_err(Error::Directory)?;
    let bin = dir.join("bin");
    let binaries = install::ensure(&bin).await?;
    let turn = file_lock::wait_exclusive(bin.join("locks").join("init.lock")).await.map_err(Error::Lock)?;
    let password = cluster::password(&dir).await?;
    cluster::init(&dir, &binaries).await?;
    let data = dir.join(cluster::DATA);
    postmaster::stop(&binaries, &data).await?;
    let (mut child, port) = postmaster::start(&binaries, &data, max_connections).await?;
    if let Err(error) = postmaster::ready(&mut child, port).await {
        let _ = postmaster::stop(&binaries, &data).await;
        let _ = child.wait().await;
        return Err(Error::Postmaster(error));
    }
    drop(turn);
    announce(&Ready::new(&password, port)).await?;
    let line = pin!(shutdown_line());
    let signalled = pin!(signal());
    let told = pin!(future::select(line, signalled));
    let exited_on_its_own = {
        let exited = pin!(child.wait());
        match future::select(told, exited).await {
            Either::Left(_) => None,
            Either::Right((status, _)) => Some(status),
        }
    };
    match exited_on_its_own {
        None => {
            postmaster::stop(&binaries, &data).await?;
            let _ = child.wait().await;
            Ok(())
        }
        Some(status) => Err(Error::Exited(status.map_err(Error::Wait)?)),
    }
}

/// Write the ready line to stdout, and flush it, so a parent reading
/// a line has it.
async fn announce(ready: &Ready) -> Result<(), Error> {
    let mut line = serde_json::to_vec(ready).map_err(Error::Announce)?;
    line.push(b'\n');
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&line).await.map_err(Error::Stdout)?;
    stdout.flush().await.map_err(Error::Stdout)
}

/// Resolves when a line of stdin is [`Command::Shutdown`]. Any other
/// line is ignored; the end of stdin, or a stdin that cannot be read,
/// is waited on forever, since neither is an instruction.
async fn shutdown_line() {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Ok(Command::Shutdown) = serde_json::from_str::<Command>(&line) {
                    return;
                }
            }
            Ok(None) | Err(_) => std::future::pending::<()>().await,
        }
    }
}

/// Resolves on Ctrl-C, and on Unix on SIGTERM too, which is what a
/// service manager sends.
async fn signal() {
    #[cfg(unix)]
    {
        let interrupt = pin!(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        let terminate = pin!(async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut terminate) => {
                    terminate.recv().await;
                }
                Err(_) => std::future::pending().await,
            }
        });
        future::select(interrupt, terminate).await;
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
