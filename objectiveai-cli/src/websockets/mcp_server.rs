//! In-process `objectiveai-mcp` server spawned at the top of
//! `instance::run`. Binds to `127.0.0.1:0` (OS-assigned port) and
//! shares the bound port back to the conduit via a
//! `Shared<oneshot::Receiver<u16>>` — any number of consumers can
//! `clone().await` to learn the port once the listener is up.

use futures::FutureExt;
use futures::future::Shared;
use tokio::sync::oneshot;

use crate::context::Context;
use crate::executor::CliCommandExecutor;

/// Handle to the in-process `objectiveai-mcp` server. `port`
/// resolves to the OS-assigned port the listener picked up; the
/// underlying serve task lives for the lifetime of the instance
/// subprocess.
#[derive(Clone)]
pub struct McpServerHandle {
    pub port: Shared<oneshot::Receiver<u16>>,
}

/// Spawn the server. Returns immediately with the handle; the
/// shared oneshot resolves after `objectiveai_mcp::setup` has
/// bound the listener. `axum::serve` runs in the same spawned task.
pub fn spawn(ctx: Context) -> McpServerHandle {
    let (port_tx, port_rx) = oneshot::channel::<u16>();
    let config = objectiveai_mcp::Config {
        address: "127.0.0.1".to_string(),
        port: 0,
        suppress_output: true,
        objectiveai_dir: ctx.config.objectiveai_dir.clone(),
        objectiveai_state: ctx.config.objectiveai_state.clone(),
    };
    let executor = CliCommandExecutor::new(ctx);
    tokio::spawn(async move {
        match objectiveai_mcp::setup(config, executor).await {
            Ok((listener, app)) => {
                let addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(_) => return,
                };
                if port_tx.send(addr.port()).is_err() {
                    return;
                }
                let _ = axum::serve(listener, app).await;
            }
            Err(_) => {
                // Drop port_tx — Shared<Receiver> consumers will
                // see `Err(RecvError)`.
            }
        }
    });
    McpServerHandle {
        port: port_rx.shared(),
    }
}
