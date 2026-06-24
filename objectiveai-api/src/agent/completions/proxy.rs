//! Per-request in-process `objectiveai-mcp-proxy`.
//!
//! Each API request lazily boots its OWN proxy (bound to a random
//! `127.0.0.1` port) via [`ProxyFactory::boot`], handed that request's
//! reverse channel + queue delegate. The returned [`ProxyHandle`] lives
//! in the request's [`crate::ctx::Context`] (`proxy_cell`), so its
//! `axum::serve` task is cancelled — by the `DropGuard` — when the
//! context's last clone drops. The factory itself (config recipe +
//! optional runtime handle) is server-level and lives on the agent
//! completions `Client`.

use std::sync::Arc;

use tokio_util::sync::{CancellationToken, DropGuard};

/// A live per-request proxy: its bound URL plus a drop guard that cancels
/// the `axum::serve` task when this handle is dropped (i.e. when the
/// owning context's last clone drops).
pub struct ProxyHandle {
    pub url: String,
    /// This request's dropper. The agent-completions client invokes it to
    /// ban + tear down a `response_id`'s upstream connections (and CLI
    /// plugin subprocesses) the moment they become unneeded — non-selected
    /// agent options at lock-in, and the selected agent after its final
    /// chunk. Wired into the proxy in [`objectiveai_mcp_proxy::setup`].
    pub dropper: objectiveai_mcp_proxy::Dropper,
    _shutdown: DropGuard,
}

impl std::fmt::Debug for ProxyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyHandle")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

/// Server-level recipe for booting a per-request proxy. Holds the static
/// config builder + an optional runtime handle (tests anchor the serve
/// task on a process-wide runtime so it isn't aborted when an individual
/// `#[tokio::test]` runtime drops). Lives on the agent completions
/// `Client`; cheap to share.
pub struct ProxyFactory {
    /// Optional runtime handle the serve task is spawned on. `None` →
    /// ambient `tokio::spawn` (correct in production).
    handle: Option<tokio::runtime::Handle>,
    config_builder_fn:
        Box<dyn Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static>,
}

impl ProxyFactory {
    /// `config_builder_fn` is called once per boot. The address / port /
    /// suppress_output knobs are overridden inside [`Self::boot`].
    pub fn new<F>(config_builder_fn: F) -> Self
    where
        F: Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static,
    {
        Self {
            handle: None,
            config_builder_fn: Box::new(config_builder_fn),
        }
    }

    /// Same as `new`, but the serve task is spawned on `handle`. Use when
    /// the caller's runtime might outlive the request (e.g. `#[tokio::test]`).
    pub fn new_with_handle<F>(handle: tokio::runtime::Handle, config_builder_fn: F) -> Self
    where
        F: Fn() -> objectiveai_mcp_proxy::ConfigBuilder + Send + Sync + 'static,
    {
        Self {
            handle: Some(handle),
            config_builder_fn: Box::new(config_builder_fn),
        }
    }

    /// Boot a fresh proxy: bind `127.0.0.1:0`, serve it on a task, and
    /// return its [`ProxyHandle`]. `reverse_channel` (per-request, `Some`
    /// for WS-attached requests) lets `ws://` upstreams speak the
    /// reverse-channel protocol directly; `queue_delegate` is this
    /// request's delegate, passed into the proxy.
    pub async fn boot(
        &self,
        reverse_channel: Option<objectiveai_mcp_proxy::ReverseChannel>,
        queue_delegate: Arc<super::queue_delegate::ApiQueueDelegate>,
    ) -> std::io::Result<Arc<ProxyHandle>> {
        let mut builder = (self.config_builder_fn)();
        builder.address = Some("127.0.0.1".into());
        builder.port = Some(0);
        builder.suppress_output = Some(true);
        let config = builder.build();

        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let (addr_tx, addr_rx) =
            tokio::sync::oneshot::channel::<std::io::Result<std::net::SocketAddr>>();

        let delegate: Arc<dyn objectiveai_mcp_proxy::QueueDelegate> = queue_delegate;
        // This request's dropper. A clone is wired into the proxy via
        // `setup`; the original is stashed on the returned handle so the
        // agent-completions client can invoke it.
        let dropper = objectiveai_mcp_proxy::Dropper::new();
        let dropper_for_task = dropper.clone();
        let task = async move {
            match objectiveai_mcp_proxy::setup(
                config,
                Some(delegate),
                reverse_channel,
                Some(dropper_for_task),
            )
            .await
            {
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
            dropper,
            _shutdown: cancel.drop_guard(),
        }))
    }
}
