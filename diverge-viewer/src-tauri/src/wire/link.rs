//! The link, as Rust holds it between uses.

use std::sync::Arc;

use diverge_sdk::wire::client::handle::{Handle, SendError};
use diverge_sdk::wire::proxy::Refusal;
use diverge_viewer_config::Daemon;
use tokio::sync::Mutex;

use super::{WireStatus, dial};

/// What the link says when the socket ended under it.
const ENDED: &str = "the connection to the daemon ended";

/// The link as it stands: not yet dialled, up, or down with the reason.
enum Link {
    Idle,
    /// Up, with the number of the dial that made it: the connection's
    /// end names its dial, so an older connection's end never takes
    /// down a newer one.
    Up { handle: Handle, dialled: u64 },
    Down(String),
}

/// The link, and how many dials have been made.
struct State {
    link: Link,
    dials: u64,
}

/// The daemon the viewer dials, and the one connection to it.
///
/// The rules, which are a socket client's:
///
/// 1. Nothing is dialled until the first use; the first use dials.
/// 2. A connection that is up is the one every send goes on.
/// 3. A connection that ended stays ended: the link is down, every
///    send on it is refused `connection` with the reason, and no send
///    dials again. The link goes down when the router's read loop
///    ends — the socket closed, however it closed — and when a send
///    finds the connection or its router gone first.
/// 4. The page dials again by asking: [`reset`](Wire::reset) puts the
///    link back to not yet dialled, and the next use dials.
///
/// Every command holds this through Tauri's state.
pub struct Wire {
    daemon: Daemon,
    state: Arc<Mutex<State>>,
}

impl Wire {
    /// A link to `daemon`, not yet dialled.
    pub fn new(daemon: Daemon) -> Wire {
        Wire { daemon, state: Arc::new(Mutex::new(State { link: Link::Idle, dials: 0 })) }
    }

    /// The handle: the connection that is up; or the one dialled now,
    /// when nothing has been dialled yet; or, when the connection
    /// ended, the refusal that says so. The lock is held through the
    /// dial, so uses that race the first wait for its one connection
    /// rather than making their own. The router runs on a task of its
    /// own, and its end takes this dial's link down.
    pub async fn handle(&self) -> Result<Handle, Refusal> {
        let mut state = self.state.lock().await;
        match &state.link {
            Link::Up { handle, .. } => return Ok(handle.clone()),
            Link::Down(message) => return Err(Refusal::Connection(message.clone())),
            Link::Idle => {}
        }
        match dial(&self.daemon.address, &self.daemon.key).await {
            Ok((handle, ran)) => {
                state.dials += 1;
                let dialled = state.dials;
                state.link = Link::Up { handle: handle.clone(), dialled };
                let shared = Arc::clone(&self.state);
                tauri::async_runtime::spawn(async move {
                    ran.await;
                    let mut state = shared.lock().await;
                    if matches!(&state.link, Link::Up { dialled: up, .. } if *up == dialled) {
                        state.link = Link::Down(ENDED.to_string());
                    }
                });
                Ok(handle)
            }
            Err(error) => {
                let message = error.to_string();
                state.link = Link::Down(message.clone());
                Err(Refusal::Connection(message))
            }
        }
    }

    /// What a send came back with, as a command answers it. A send
    /// that found the connection or its router gone takes the link
    /// down with its reason; a scope that is not open says nothing
    /// about the connection and leaves it as it is.
    pub async fn sent<T>(&self, result: Result<T, SendError>) -> Result<T, Refusal> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let refusal = Refusal::from(error);
                if let Refusal::Connection(message) = &refusal {
                    let mut state = self.state.lock().await;
                    if matches!(state.link, Link::Up { .. }) {
                        state.link = Link::Down(message.clone());
                    }
                }
                Err(refusal)
            }
        }
    }

    /// How the link stands, without dialling: `dialling` while a use
    /// holds the lock, which only a dial does for long.
    pub fn status(&self) -> WireStatus {
        match self.state.try_lock() {
            Err(_) => WireStatus::Dialling,
            Ok(state) => match &state.link {
                Link::Idle => WireStatus::Idle,
                Link::Up { .. } => WireStatus::Connected,
                Link::Down(message) => WireStatus::Failed { message: message.clone() },
            },
        }
    }

    /// Put the link back to not yet dialled, whatever it was: the next
    /// use dials. The page's one way to dial again after the connection
    /// ended, and its remedy for one that is up in name and answers
    /// nothing.
    pub async fn reset(&self) {
        self.state.lock().await.link = Link::Idle;
    }
}
