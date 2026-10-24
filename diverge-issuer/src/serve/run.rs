//! The issuer's life: start, serve, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::file_lock;
use futures_util::future::{self, Either};
use tokio::sync::watch;

use crate::config::Config;
use crate::endpoints;
use crate::signing::Signing;
use crate::store::Store;

use super::{Error, Issuer, listen, shutdown};

/// Run the issuer on `config`, the file at `root`, until Ctrl-C or,
/// on Unix, SIGTERM — or until a loopback `POST /stop` on the port,
/// which is the same stop (see [`stop`](super::stop)). In order:
///
/// 1. The root is locked: one issuer per root, the lock file stamped
///    with this process's id, and a root another issuer holds refused
///    at once, naming the holder's id where it left one.
/// 2. `issuer.sqlite` is opened, made where it is absent.
/// 3. The signing keys are read from `issuer.sqlite`, one minted
///    where there is none — see [`signing`](crate::signing) — and the
///    discovery document and the key set made from the configuration
///    and the keys.
/// 4. The port is listened on until the signal or the route sends the
///    stop; then the listener drains, and the lock is let go, last.
///
/// A port that could not be bound is the error.
pub async fn run(config: Config, root: PathBuf) -> Result<(), Error> {
    let lock = root.join(crate::LOCK_FILE);
    let Some(held) = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)? else {
        let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
        return Err(Error::Locked { root, holder });
    };
    held.write(std::process::id().to_string().into_bytes()).await.map_err(Error::Lock)?;
    let store = Store::open(&root).await.map_err(Error::Store)?;
    let signing = Signing::load(&store).await.map_err(Error::Signing)?;
    let discovery = endpoints::document(&config).map_err(Error::Discovery)?;
    let key_set = serde_json::to_string(&signing.key_set()).map_err(|error| Error::Discovery(error.to_string()))?;
    let port = config.port;
    let (stop, _) = watch::channel(false);
    let issuer = Arc::new(Issuer { config, store, signing, discovery, key_set, stop });
    let listening = pin!(listen(Arc::clone(&issuer), port, issuer.stop.subscribe()));
    let signal = pin!(shutdown());
    let outcome = match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            issuer.stop.send_replace(true);
            listening.await
        }
    };
    drop(held);
    outcome
}
