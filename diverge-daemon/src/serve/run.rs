//! The daemon's life: start, listen, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use crate::config::Config;
use crate::judge::DaemonAuthenticator;
use diverge_sdk::file_lock;
use diverge_sdk::identities::{KeySets, Own};
use futures_util::future::{self, Either};
use tokio::sync::watch;

use super::{Error, listen};
use crate::containers;
use crate::daemon::Daemon;
use crate::database;
use crate::postgres;
use crate::providers;
use crate::store::{self, vault};

/// How long an issuer's key set is held where its answer names no
/// `max-age`.
const KEY_SETS: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Run the daemon on `config`, the file at `root`, under
/// `<root>/daemon/`, until Ctrl-C or, on Unix, SIGTERM — or until a
/// loopback `POST /stop` on the port, which is the same stop (see
/// [`stop`](super::stop)).
///
/// In order: the root is locked — one daemon per root, the lock file
/// stamped with this process's id, and a root another daemon holds
/// refused at once, before anything is made or started, naming the
/// holder's id where it left one; a lock file a dead daemon left is
/// taken like any other, since the kernel let its lock go with it —
/// the directory is made; the database the configuration names is
/// started, or named; the daemon's identities are opened; the store
/// is opened on it, its schema applied; the daemon is built, its stop
/// made here so the signal and the route send on the one sender;
/// every vault slot a run of an earlier daemon held is freed, since
/// no run survives a restart; the port is listened on until the
/// signal or the route sends the stop, every provider the
/// configuration's `providers.connect` names dialled meanwhile; the listener drains; every container run
/// is stopped; the dials are ended, and their connections with them;
/// the database that was started is stopped; and the lock is let go,
/// last. The signal is armed only once the port is listened on, and
/// the route exists only then too, so a stop asked during a slow
/// start is refused by the kernel, not by the daemon. A store that
/// cannot be opened stops the database it just started before the
/// error is returned.
pub async fn run(config: Config, root: PathBuf) -> Result<(), Error> {
    let lock = root.join(diverge_sdk::daemon::LOCK_FILE);
    let Some(held) = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)? else {
        let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
        return Err(Error::Locked { root, holder });
    };
    held.write(std::process::id().to_string().into_bytes()).await.map_err(Error::Lock)?;
    let dir = root.join("daemon");
    tokio::fs::create_dir_all(&dir).await.map_err(Error::Directory)?;
    let key_sets = KeySets::new(KEY_SETS).map_err(Error::KeySets)?;
    let own = Own::new(&root, config.identities.clone(), key_sets).await.map_err(Error::Identities)?;
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
    let (stop, _) = watch::channel(false);
    let authenticator = Arc::new(DaemonAuthenticator::new(own, config.providers.accepted.clone(), store.clone()));
    let daemon = Arc::new(Daemon::new(
        store,
        authenticator,
        config.providers.clone(),
        config.daemons.clone(),
        logs,
        std::time::Duration::from_secs(config.idle_seconds),
        database,
        config.accept_daemons,
        std::time::Duration::from_secs(config.capacity_poll_seconds),
        stop,
    ));
    if let Err(error) = clear_holders(&daemon).await {
        postgres.stop().await;
        return Err(Error::Store(error));
    }
    providers::dial_all(&daemon).await;
    if let Err(error) = containers::services::boot(&daemon).await {
        postgres.stop().await;
        return Err(Error::Store(error));
    }
    let listening = pin!(listen(config.port, daemon.stop.subscribe(), Arc::clone(&daemon)));
    let signal = pin!(shutdown());
    let outcome = match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            daemon.stop.send_replace(true);
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
