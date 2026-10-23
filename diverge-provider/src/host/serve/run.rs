//! The provider's life: start, serve, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::file_lock;
use diverge_sdk::keyring::Keyring;
use futures_util::future::{self, Either};
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::config::Config;
use crate::host::keys::Keys;

use super::{Error, Provider, dial, listen};

/// Run the provider on `config`, the file at `root`,
/// under `<root>/provider/`, until Ctrl-C or, on Unix, SIGTERM — or
/// until a loopback `POST /stop` on the port, which is the same stop
/// (see [`stop`](super::stop)). First
/// the root is locked — one provider per root, the lock file stamped
/// with this process's id, and a root another provider holds refused
/// at once, before anything on disk is swept or started, naming the
/// holder's id where it left one; a lock file a dead provider left is
/// taken like any other, since the kernel let its lock go with it.
/// The keys the document names are read from the root's keyring, and
/// an entry it does not hold stops the start.
/// The stop is made, and the pieces are built around it by
/// [`Provider::start`]; one task per
/// peer of `clients.unbrokered` dials; the port is listened on until
/// the signal or the route sends the stop, and then, in order: the
/// listener drains; every session, accepted or dialled, is dropped on
/// the same stop, so every socket closes; every dial task is ended;
/// every container and loop mount of this provider is swept away; the
/// provider is dropped — the registry's server and the machine's
/// tunnel with it; and the lock is let go, last. The signal is armed
/// only once the port is listened on, and the route exists only then
/// too, so a stop asked during a slow start is refused by the kernel,
/// not by the provider. A port that could not be bound is the error,
/// after the same sweep.
pub async fn run(config: Config, root: PathBuf) -> Result<(), Error> {
    let lock = root.join(diverge_sdk::provider::LOCK_FILE);
    let Some(held) = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)? else {
        let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
        return Err(Error::Locked { root, holder });
    };
    held.write(std::process::id().to_string().into_bytes()).await.map_err(Error::Lock)?;
    let dir = root.join("provider");
    let port = config.port;
    let peers = config.clients.as_ref().and_then(|clients| clients.unbrokered.clone()).unwrap_or_default();
    let keyring = Keyring::open(&root).await.map_err(Error::Keyring)?;
    let keys = Arc::new(Keys::resolve(&config, &keyring).await.map_err(Error::Keys)?);
    let (stop, _) = watch::channel(false);
    let provider = Arc::new(Provider::start(config, dir, stop, keys).await?);
    let mut dials = JoinSet::new();
    for peer in peers {
        dials.spawn(dial(Arc::clone(&provider), peer));
    }
    let listening = pin!(listen(Arc::clone(&provider), port, provider.stop.subscribe()));
    let signal = pin!(shutdown());
    let outcome = match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            provider.stop.send_replace(true);
            listening.await
        }
    };
    dials.shutdown().await;
    provider.deployer.sweep().await;
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
