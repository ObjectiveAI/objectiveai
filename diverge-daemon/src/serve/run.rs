//! The daemon's life: start, listen, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::config::daemon::Config;
use futures_util::future::{self, Either};
use tokio::sync::watch;

use super::{Error, listen};
use crate::containers;
use crate::daemon::Daemon;
use crate::database;
use crate::postgres;
use crate::providers;
use crate::store;

/// Run the daemon on `config`, its block of the file at `root`, under
/// `<root>/daemon/`, until Ctrl-C or, on Unix, SIGTERM.
///
/// In order: the directory is made; the database the configuration names is started, or
/// named; the store is opened on it, its schema applied and root
/// seeded into a fresh one; the daemon is built; the port is listened
/// on until the signal, every outgoing provider on record dialled
/// meanwhile; the listener drains; the dials are ended, and their
/// connections with them; and the database that was started is
/// stopped. A store that cannot be opened stops the
/// database it just started before the error is returned.
pub async fn run(config: Config, root: PathBuf) -> Result<(), Error> {
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
    ));
    if let Err(error) = providers::dial_all(&daemon).await {
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
