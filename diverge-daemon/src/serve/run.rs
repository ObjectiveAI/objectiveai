//! The daemon's life: start, listen, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use crate::config::Config;
use diverge_sdk::file_lock;
use futures_util::future::{self, Either};
use tokio::sync::watch;

use super::{Error, listen};
use crate::containers;
use crate::daemon::Daemon;
use crate::database;
use crate::postgres;
use crate::providers;
use crate::store::{self, vault};

/// Run the daemon on `config`, the file at `root`, under
/// `<root>/daemon/`, until Ctrl-C or, on Unix, SIGTERM.
///
/// In order: the root is locked — one daemon per root, the lock file
/// stamped with this process's id, and a root another daemon holds
/// refused at once, before anything is made or started, naming the
/// holder's id where it left one; a lock file a dead daemon left is
/// taken like any other, since the kernel let its lock go with it —
/// the directory is made; the database the configuration names is
/// started, or named; the store is opened on it, its schema applied
/// and root seeded into a fresh one; the daemon is built; every vault
/// slot a run of an earlier daemon held is freed, since no run
/// survives a restart; the port is listened on until the signal,
/// every outgoing provider on record dialled meanwhile; the listener
/// drains; the dials are ended, and their connections with them; the
/// database that was started is stopped; and the lock is let go,
/// last. A store that cannot be opened stops the database it just
/// started before the error is returned.
pub async fn run(config: Config, root: PathBuf) -> Result<(), Error> {
    let lock = root.join(diverge_sdk::daemon::LOCK_FILE);
    let Some(held) = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)? else {
        let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
        return Err(Error::Locked { root, holder });
    };
    held.write(std::process::id().to_string().into_bytes()).await.map_err(Error::Lock)?;
    let dir = root.join("daemon");
    tokio::fs::create_dir_all(&dir).await.map_err(Error::Directory)?;
    let (postgres, url) = postgres::start(&config.postgres, &root).await.map_err(Error::Postgres)?;
    let store = match store::open(&url, postgres.is_local()).await {
        Ok(store) => store,
        Err(error) => {
            postgres.stop().await;
            return Err(Error::Store(error));
        }
    };
    let logs = dir.join("agents");
    if let Err(error) = tokio::fs::create_dir_all(&logs).await {
        postgres.stop().await;
        return Err(Error::Directory(error));
    }
    let database = match database::Target::parse(&url, postgres.is_local()) {
        Ok(database) => database,
        Err(error) => {
            postgres.stop().await;
            return Err(Error::Database(error));
        }
    };
    let daemon = Arc::new(Daemon::new(
        store,
        logs,
        std::time::Duration::from_secs(config.idle_seconds),
        database,
        config.accept_daemons,
        std::time::Duration::from_secs(config.capacity_poll_seconds),
    ));
    if let Err(error) = clear_holders(&daemon).await {
        postgres.stop().await;
        return Err(Error::Store(error));
    }
    if let Err(error) = providers::dial_all(&daemon).await {
        postgres.stop().await;
        return Err(Error::Store(error));
    }
    if let Err(error) = containers::services::boot(&daemon).await {
        postgres.stop().await;
        return Err(Error::Store(error));
    }
    let (stop, stopped) = watch::channel(false);
    let listening = pin!(listen(config.port, stopped, Arc::clone(&daemon)));
    let signal = pin!(shutdown());
    let outcome = match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            let _ = stop.send(true);
            listening.await
        }
    };
    containers::stop_all(&daemon).await;
    daemon.live.stop_dials().await;
    postgres.stop().await;
    drop(held);
    outcome
}

/// Resolves on Ctrl-C, and on Unix on SIGTERM too, which is what a
/// service manager sends.
async fn shutdown() {
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

/// Every vault slot freed of its holder: what a run of an earlier
/// daemon held, which no run of this one does. A store that refuses
/// is the boot's failure.
async fn clear_holders(daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    vault::clear_holders(&mut conn).await?;
    Ok(())
}
