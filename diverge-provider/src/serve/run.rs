//! The provider's life: start, serve, stop.

use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};
use tokio::sync::watch;
use tokio::task::JoinSet;

use super::{Error, Provider, dial, listen};
use crate::config::Config;

/// Run the provider on `config`, in `dir`, until Ctrl-C or, on Unix,
/// SIGTERM. The pieces are built by [`Provider::start`]; one task per
/// peer of `clients.unbrokered` dials; the port is listened on until
/// the signal, and then the listener drains, every dial task is
/// ended, every container and loop mount of this provider is swept
/// away, and the provider is dropped — the registry's server and the
/// machine's tunnel with it. A port that could not be bound is the
/// error, after the same sweep.
pub async fn run(config: Config, dir: PathBuf) -> Result<(), Error> {
    let port = config.port;
    let peers = config.clients.as_ref().and_then(|clients| clients.unbrokered.clone()).unwrap_or_default();
    let provider = Arc::new(Provider::start(config, dir).await?);
    let (stop, stopped) = watch::channel(false);
    let mut dials = JoinSet::new();
    for peer in peers {
        dials.spawn(dial(Arc::clone(&provider), peer));
    }
    let listening = pin!(listen(Arc::clone(&provider), port, stopped));
    let signal = pin!(shutdown());
    let outcome = match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            let _ = stop.send(true);
            listening.await
        }
    };
    dials.shutdown().await;
    provider.deployer.sweep().await;
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
