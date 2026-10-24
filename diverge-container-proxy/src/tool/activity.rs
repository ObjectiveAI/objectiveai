//! The tool container's activity: the MCP exchanges in flight, and
//! the gate they wait behind.

use diverge_sdk::container_proxy::outside::endpoints::tools::begin::server::response;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::{Mutex, watch};

use crate::encode::encoded;
use crate::program;
use crate::proxy::Proxy;

/// How many MCP exchanges are in flight, and whether the program
/// holds secrets fed since the last of them went out.
///
/// An exchange entering with none in flight is the container's
/// `Active`; the last leaving is its `Inactive`, carrying the
/// rotating secrets collected from the program. Between an
/// `Inactive` and the next `Fed` the gate is closed — when the
/// program takes any secret; a program that takes none has a gate
/// that is never closed — and an exchange that entered waits behind
/// it rather than reach a program whose secrets are not its own yet.
/// The count is one lock, held across the word written for a change
/// of it — `Active`, `Inactive` — and across the collect before an
/// `Inactive`, so the two words alternate on the wire as the count
/// does; it is never held while waiting for the gate, which only a
/// feed opens.
pub struct Activity {
    inflight: Mutex<usize>,
    fed: watch::Sender<bool>,
}

impl Activity {
    pub fn new() -> Self {
        Activity {
            inflight: Mutex::new(0),
            fed: watch::Sender::new(false),
        }
    }

    /// The gate's state at `Begun`: open for a program that takes no
    /// secret, closed until the first feed for one that does.
    pub fn set_open(&self, open: bool) {
        self.fed.send_replace(open);
    }

    /// Open it: the program has applied a feed. Idempotent.
    pub fn open(&self) {
        self.fed.send_replace(true);
    }

    /// Close it: the program's rotating secrets were collected, and
    /// what it holds is not its to use until the next feed.
    pub fn close(&self) {
        self.fed.send_replace(false);
    }

    /// Wait until it is open; at once if it already is.
    pub async fn opened(&self) {
        let mut watcher = self.fed.subscribe();
        let _ = watcher.wait_for(|open| *open).await;
    }

    /// One exchange begins: the count up, `Active` on the main stream
    /// when it went from none to one — under the lock — and then,
    /// with the lock released, the wait for the gate.
    pub async fn enter(&self, scope: &ScopeHandle) {
        {
            let mut inflight = self.inflight.lock().await;
            *inflight += 1;
            if *inflight == 1 {
                if let Some(payload) = encoded(&response::Frame::Active) {
                    scope.send_response(&payload).await;
                }
            }
        }
        self.opened().await;
    }

    /// One exchange is over, its reply written: the count down, and
    /// when it went from one to none — under the lock, so no exchange
    /// enters between the collect and the word — the rotating secrets
    /// are collected from the program (not asked for when it declared
    /// none), the gate closed when the program takes any secret, and
    /// `Inactive` written with the values; a collect that failed is
    /// the `Error` that ends the run, naming no value, after which the
    /// begin loop finishes the scope.
    pub async fn leave(&self, proxy: &Proxy, scope: &ScopeHandle) {
        let mut inflight = self.inflight.lock().await;
        *inflight -= 1;
        if *inflight > 0 {
            return;
        }
        let names = proxy.rotating();
        let collected = if names.is_empty() {
            Ok(Values::new())
        } else {
            program::collect(&proxy.upstream, &names).await
        };
        match collected {
            Ok(values) => {
                if proxy.gated() {
                    self.close();
                }
                if let Some(payload) = encoded(&response::Frame::Inactive(values)) {
                    scope.send_response(&payload).await;
                }
            }
            Err(error) => {
                if let Some(payload) = encoded(&response::Frame::Error(error)) {
                    scope.send_response(&payload).await;
                }
                proxy.end();
            }
        }
    }
}
