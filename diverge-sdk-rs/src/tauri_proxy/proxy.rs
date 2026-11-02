//! What the app manages.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};

use crate::wire::handshake::Authenticator;
use crate::wire::proxy::{Link, Redial};

/// The state the commands read: one [`Link`] to one daemon. The app
/// manages exactly one, with `tauri::Builder::manage`; the commands
/// fail to resolve it otherwise.
pub struct TauriProxy(Link, Arc<Mutex<HashMap<u32, Arc<Semaphore>>>>);

impl TauriProxy {
    /// The daemon at `address`, its handshake made from `issuer`
    /// through `authenticator` — the app's identity at that issuer,
    /// from the host's [`identities`](crate::identities), and its
    /// judgment of the daemon's — dialled again as `redial` says. An
    /// address with no scheme is dialled as `ws://<address>/`.
    ///
    /// With [`Redial::Every`] the link's [`redial`](Link::redial) is
    /// spawned here, on Tauri's runtime, so the daemon is dialled at
    /// once; with [`Redial::Never`] nothing is, and the page's first
    /// use dials. Made inside the app's `setup`, where that runtime
    /// runs.
    pub fn new(address: impl Into<String>, issuer: impl Into<String>, authenticator: Arc<dyn Authenticator>, redial: Redial) -> TauriProxy {
        let link = Link::new(address, issuer, authenticator, redial);
        if let Redial::Every(_) = redial {
            tauri::async_runtime::spawn(link.redial());
        }
        TauriProxy(link, Arc::default())
    }

    /// The link the commands use.
    pub fn link(&self) -> &Link {
        &self.0
    }

    /// A page is loading: the page before it, if any, is gone, and so
    /// is everything it had open.
    ///
    /// The rules:
    ///
    /// 1. Every channel still forwarded to the page is let go: its
    ///    forwarding ends and its receiver is dropped, as
    ///    `wire_dropped` does — the page that would have taken the
    ///    next frame is gone, and nothing else will.
    /// 2. The connection is reset ([`Link::reset`]): it is closed, so
    ///    the daemon ends every scope the old page had open — a watch,
    ///    a console, a held container — and the new page starts on a
    ///    connection of its own.
    ///
    /// The app calls it when a page starts loading, the first one
    /// included, which finds nothing to let go.
    pub async fn page_loading(&self) {
        // Rule 1.
        for (_, took) in self.1.lock().await.drain() {
            took.close();
        }
        // Rule 2.
        self.0.reset().await;
    }

    /// The page's channels being forwarded, by their ids: each one's
    /// permits, one given each time the page takes a frame.
    pub(super) fn pumps(&self) -> &Arc<Mutex<HashMap<u32, Arc<Semaphore>>>> {
        &self.1
    }
}
