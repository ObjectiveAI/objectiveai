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
    /// Optional runtime handle used to anchor the proxy's `axum::serve`
    /// task. `None` means "use the ambient runtime via `tokio::spawn`",
    /// which is correct in production (single long-lived runtime). Tests
    /// pass `Some(handle)` pointing at a process-wide runtime so the
    /// listener task isn't aborted when an individual `#[tokio::test]`
    /// runtime drops.
    handle: Option<tokio::runtime::Handle>,
    config_builder_fn:
        Box<dyn Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static>,
    /// In-process queue delegate the proxy uses to surface
    /// pending `message_queue` content on tool responses. Owned
    /// here (not by the OnceCell) because `run_agent_loop` needs
    /// to call `register` / `confirm` / `unregister` on it
    /// regardless of whether the proxy has been booted yet.
    queue_delegate: Arc<super::queue_delegate::ApiQueueDelegate>,
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
            handle: None,
            config_builder_fn: Box::new(config_builder_fn),
            queue_delegate: Arc::new(super::queue_delegate::ApiQueueDelegate::new()),
        }
    }

    /// Same as `new`, but the proxy's `axum::serve` task is spawned on
    /// the supplied runtime handle. Use this when the caller's runtime
    /// might outlive the spawner (e.g. `#[tokio::test]` runtimes that
    /// drop at end-of-test).
    pub fn new_with_handle<F>(handle: tokio::runtime::Handle, config_builder_fn: F) -> Self
    where
        F: Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static,
    {
        Self {
            cell: OnceCell::new(),
            handle: Some(handle),
            config_builder_fn: Box::new(config_builder_fn),
            queue_delegate: Arc::new(super::queue_delegate::ApiQueueDelegate::new()),
        }
    }

    /// The in-process queue delegate this spawner installed into
    /// the proxy. `run_agent_loop` calls `register` /
    /// `confirm` / `drain_undelivered` / `unregister` on this
    /// across its lifetime.
    pub fn queue_delegate(&self) -> Arc<super::queue_delegate::ApiQueueDelegate> {
        self.queue_delegate.clone()
    }

    pub async fn get(&self) -> std::io::Result<Arc<ProxyHandle>> {
        self.cell
            .get_or_try_init(|| async {
                let mut builder = (self.config_builder_fn)();
                builder.address = Some("127.0.0.1".into());
                builder.port = Some(0);
                builder.suppress_output = Some(true);
                let config = builder.build();

                let cancel = CancellationToken::new();
                let token = cancel.clone();
                let (addr_tx, addr_rx) =
                    tokio::sync::oneshot::channel::<std::io::Result<std::net::SocketAddr>>();

                let delegate: Arc<dyn objectiveai_mcp_proxy::QueueDelegate> =
                    self.queue_delegate.clone();
                let task = async move {
                    match objectiveai_mcp_proxy::setup(config, Some(delegate)).await {
                        Ok((listener, router)) => {
                            let addr = listener.local_addr();
                            let send_result = match addr {
                                Ok(a) => addr_tx.send(Ok(a)),
                                Err(e) => {
                                    let _ = addr_tx.send(Err(e));
                                    return;
                                }
                            };
                            if send_result.is_err() {
                                // Caller dropped before we sent the addr; bail.
                                return;
                            }
                            let _ = axum::serve(listener, router)
                                .with_graceful_shutdown(token.cancelled_owned())
                                .await;
                        }
                        Err(e) => {
                            let _ = addr_tx.send(Err(e));
                        }
                    }
                };

                match &self.handle {
                    Some(h) => {
                        h.spawn(task);
                    }
                    None => {
                        tokio::spawn(task);
                    }
                }

                let addr = addr_rx
                    .await
                    .map_err(|_| std::io::Error::other("proxy task dropped before reporting addr"))??;

                Ok(Arc::new(ProxyHandle {
                    url: format!("http://{addr}"),
                    _shutdown: cancel.drop_guard(),
                }))
            })
            .await
            .map(Arc::clone)
    }
}
