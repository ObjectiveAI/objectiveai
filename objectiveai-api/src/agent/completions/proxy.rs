//! Lazy in-process `objectiveai-mcp-proxy`.
//!
//! Each agent-completions request that needs MCP routes through one
//! shared, program-lifetime instance of the proxy bound to a random
//! `127.0.0.1` port. The proxy is built via the proxy crate's `setup()`
//! library function and served on a tokio task; dropping the spawner's
//! handle cancels that task.

use std::sync::Arc;

use tokio::sync::OnceCell;
use tokio_util::sync::{CancellationToken, DropGuard};

/// Live proxy instance: its bound URL plus a drop guard that cancels the
/// `axum::serve` task when this handle is dropped (i.e. when the spawner
/// goes away).
pub struct ProxyHandle {
    pub url: String,
    _shutdown: DropGuard,
}

/// Bootstraps the in-process proxy on first use. Concurrent first
/// callers race on `OnceCell`; the loser awaits the winner's result.
pub struct ProxySpawner {
    cell: OnceCell<Arc<ProxyHandle>>,
    config_builder_fn:
        Box<dyn Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static>,
}

impl ProxySpawner {
    /// `config_builder_fn` is called once on first init. The address /
    /// port / suppress_output knobs are overridden inside `get()` so the
    /// caller can ignore them.
    pub fn new<F>(config_builder_fn: F) -> Self
    where
        F: Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static,
    {
        Self {
            cell: OnceCell::new(),
            config_builder_fn: Box::new(config_builder_fn),
        }
    }

    pub async fn get(&self) -> std::io::Result<Arc<ProxyHandle>> {
        self.cell
            .get_or_try_init(|| async {
                let mut builder = (self.config_builder_fn)();
                builder.address = Some("127.0.0.1".into());
                builder.port = Some(0);
                builder.suppress_output = Some(true);
                let config = builder.build();

                let (listener, router) = objectiveai_mcp_proxy::setup(config).await?;
                let addr = listener.local_addr()?;

                let cancel = CancellationToken::new();
                let token = cancel.clone();
                tokio::spawn(async move {
                    let _ = axum::serve(listener, router)
                        .with_graceful_shutdown(token.cancelled_owned())
                        .await;
                });

                Ok(Arc::new(ProxyHandle {
                    url: format!("http://{addr}"),
                    _shutdown: cancel.drop_guard(),
                }))
            })
            .await
            .map(Arc::clone)
    }
}
