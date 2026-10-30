//! What every part of the proxy shares.

use std::sync::Arc;

use diverge_sdk::shared::containers::secrets::{Declaration, Mode};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::{Mutex, Notify, OnceCell, mpsc, watch};

use crate::agent::Cmd;
use crate::begin::Family;
use crate::filesystem::mount::Mounts;
use crate::inside::mcp::{Gate, Peers};
use crate::inside::pairs::Pairs;
use crate::program::Upstream;
use crate::tool::{Activity, Sessions};

/// The begin scope, once it has begun: the scope the proxy's own asks
/// ride, and which family's frames they are.
#[derive(Clone)]
pub struct Begun {
    /// The scope, shared by every task that asks on it.
    pub scope: Arc<ScopeHandle>,
    /// Whose asks: the two families encode the same asks, each in its
    /// own frame and each less what its container never makes.
    pub family: Family,
}

/// The one proxy: the connection's latches, the begin scope once it
/// exists, the agent's driver, and the pieces each surface keeps.
///
/// Everything here is either set once or guarded; nothing is held
/// across an await.
pub struct Proxy {
    /// Whether the one connection has ever arrived. There is no
    /// second connection to a proxy.
    connected: Mutex<bool>,
    /// Whether the connection has begun. A second begin is refused.
    begun: Mutex<bool>,
    /// The begin scope, published once `Begun` has gone out; an asker
    /// waits on this until then — the park.
    begin: watch::Sender<Option<Begun>>,
    /// The agent's driver, set when an agent container begins. A tool
    /// container never sets it.
    commands: OnceCell<mpsc::UnboundedSender<Cmd>>,
    /// The secrets the program declared at registration, by name and
    /// mode: set once, before `Begun` goes out, and read by whichever
    /// gate the container has.
    declaration: OnceCell<Declaration>,
    /// The run ending by the proxy's own hand — a collect that failed
    /// — which the begin loop hears and finishes the scope on.
    ending: Notify,
    /// The client the program's server is dialled with.
    pub upstream: Upstream,
    /// The MCP sessions a tool container's server is called
    /// through, one per caller.
    pub sessions: Sessions,
    /// A tool container's MCP exchanges in flight, and the gate they
    /// wait behind. An agent container never touches it: its gate is
    /// the driver's.
    pub activity: Activity,
    /// The FUSE mounts made so far.
    pub mounts: Mounts,
    /// The database connections announced and not yet paired.
    /// The daemon connections announced and not yet paired.
    pub daemons: Pairs,
    pub pairs: Pairs,
    /// The MCP sessions the resident notifications stream addresses.
    pub peers: Peers,
    /// What the notifications ask waits behind.
    pub gate: Gate,
}

impl Proxy {
    pub fn new() -> Self {
        let (begin, _) = watch::channel(None);
        Proxy {
            connected: Mutex::new(false),
            begun: Mutex::new(false),
            begin,
            commands: OnceCell::new(),
            declaration: OnceCell::new(),
            ending: Notify::new(),
            upstream: Upstream::new(),
            sessions: Sessions::default(),
            activity: Activity::new(),
            mounts: Mounts::new(),
            daemons: Pairs::new(),
            pairs: Pairs::new(),
            peers: Peers::new(),
            gate: Gate::new(),
        }
    }

    /// Take the one connection. `false` is a connection already taken,
    /// ever: the container's life is one connection.
    pub async fn claim_connection(&self) -> bool {
        let mut connected = self.connected.lock().await;
        if *connected {
            return false;
        }
        *connected = true;
        true
    }

    /// Take the one begin. `false` is a connection that has begun.
    pub async fn claim_begin(&self) -> bool {
        let mut begun = self.begun.lock().await;
        if *begun {
            return false;
        }
        *begun = true;
        true
    }

    /// Give the begin back: the arguments were refused, and the
    /// connection has not begun after all.
    pub async fn release_begin(&self) {
        *self.begun.lock().await = false;
    }

    /// The begin scope is open: every asker parked on it goes on.
    pub fn publish(&self, begun: Begun) {
        self.begin.send_replace(Some(begun));
    }

    /// The begin scope, waiting for it if it has not begun. `None`
    /// cannot happen while the proxy exists, and is typed rather than
    /// unwrapped.
    pub async fn begun(&self) -> Option<Begun> {
        let mut receiver = self.begin.subscribe();
        match receiver.wait_for(Option::is_some).await {
            Ok(begun) => begun.clone(),
            Err(_) => None,
        }
    }

    /// The agent's driver, once an agent container has begun.
    pub fn commands(&self) -> Option<&mpsc::UnboundedSender<Cmd>> {
        self.commands.get()
    }

    /// Keep the agent's driver. A second is dropped: there is one
    /// begin, so there is one driver.
    pub fn set_commands(&self, commands: mpsc::UnboundedSender<Cmd>) {
        let _ = self.commands.set(commands);
    }

    /// Keep the program's declaration. A second is dropped: there is
    /// one registration, so there is one declaration.
    pub fn set_declaration(&self, declaration: Declaration) {
        let _ = self.declaration.set(declaration);
    }

    /// Whether the program takes any secret at all: the gate is
    /// closed at the begin, and closed again by every inactive, only
    /// then. A program that takes none has nothing to wait for, and a
    /// program that has not registered takes nothing yet.
    pub fn gated(&self) -> bool {
        self.declaration.get().is_some_and(|declaration| !declaration.is_empty())
    }

    /// The names the program declared `rotating`, in the declaration's
    /// order: what every inactive collects.
    pub fn rotating(&self) -> Vec<String> {
        self.declaration
            .get()
            .map(|declaration| {
                declaration
                    .iter()
                    .filter(|(_, mode)| **mode == Mode::Rotating)
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// End the run: the `Error` is on the main stream already, and
    /// the begin loop finishes the scope behind it.
    pub fn end(&self) {
        self.ending.notify_one();
    }

    /// The run ended by this end: resolves once [`end`](Self::end)
    /// has been called, before or after.
    pub async fn ended(&self) {
        self.ending.notified().await;
    }
}
