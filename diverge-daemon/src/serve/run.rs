//! The daemon's life: start, listen, stop.

use std::path::PathBuf;
use std::pin::pin;

use futures_util::future::{self, Either};
use tokio::sync::watch;

use super::{Error, listen};
use crate::config::Config;

/// Run the daemon on `config`, in `dir`, until Ctrl-C or, on Unix,
/// SIGTERM: the port is listened on until the signal, and then the
/// listener drains. A port that could not be bound is the error.
/// `dir` is held for what the daemon will keep there; today nothing.
pub async fn run(config: Config, dir: PathBuf) -> Result<(), Error> {
    let _dir = dir;
    let (stop, stopped) = watch::channel(false);
    let listening = pin!(listen(config.port, stopped));
    let signal = pin!(shutdown());
    match future::select(listening, signal).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(((), listening)) => {
            let _ = stop.send(true);
            listening.await
        }
    }
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
