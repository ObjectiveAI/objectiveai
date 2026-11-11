//! The signals that stop the issuer.

/// Resolves on Ctrl-C, and on Unix on SIGTERM too, which is what a
/// service manager sends.
pub async fn shutdown() {
    #[cfg(unix)]
    {
        let interrupt = std::pin::pin!(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        let terminate = std::pin::pin!(async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut terminate) => {
                    terminate.recv().await;
                }
                Err(_) => std::future::pending().await,
            }
        });
        futures_util::future::select(interrupt, terminate).await;
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
