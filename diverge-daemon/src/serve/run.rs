//! The daemon's life: start, listen, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use tokio::sync::watch;

use super::{Error, listen};
use crate::config::Config;
use crate::daemon::Daemon;
use crate::postgres;
use crate::providers;
use crate::store;

/// Run the daemon on `config`, in `dir`, until Ctrl-C or, on Unix,
/// SIGTERM.
///
/// In order: the database the configuration names is started, or
/// named; the store is opened on it, its schema applied and root
/// seeded into a fresh one; the daemon is built; the port is listened
/// on until the signal, every outgoing provider on record dialled
/// meanwhile; the listener drains; the dials are ended, and their
/// connections with them; and the database that was started is
/// stopped. A store that cannot be opened stops the
/// database it just started before the error is returned.
pub async fn run(config: Config, dir: PathBuf) -> Result<(), Error> {
    let (postgres, url) = postgres::start(&config.postgres, &dir).await.map_err(Error::Postgres)?;
    let store = match store::open(&url, postgres.is_local()).await {
        Ok(store) => store,
        Err(error) => {
            postgres.stop().await;
            return Err(Error::Store(error));
        }
    };
    let resources = dir.join("resources");
    if let Err(error) = tokio::fs::create_dir_all(resources.join("incoming")).await {
        postgres.stop().await;
        return Err(Error::Resources(error));
    }
    let daemon = Arc::new(Daemon::new(store, resources));
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
