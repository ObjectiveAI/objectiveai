//! The one connection a native side holds for its page.

use std::sync::Arc;

use tokio::sync::Mutex;

use super::{Refusal, WireStatus};
use crate::wire::client::dial::dial;
use crate::wire::client::handle::{Handle, SendError};

/// What the link says when the socket ended under it.
const ENDED: &str = "the connection to the daemon ended";

/// The link as it stands.
enum State {
    /// Not yet dialled, since the link was made or reset.
    Idle,
    /// Up, with the number of the dial that made it: the connection's
    /// end names its dial, so an older connection's end never takes
    /// down a newer one.
    Up { handle: Handle, dialled: u64 },
    /// Down, with the reason.
    Down(String),
}

/// The state, and how many dials have been made.
struct Inner {
    state: State,
    dials: u64,
}

/// A daemon's address and key, and the one connection to it, as a
/// native side holds it for a page that speaks through the proxy.
///
/// The rules, which are a socket client's:
///
/// 1. Nothing is dialled until the first use; the first use dials.
/// 2. A connection that is up is the one every send goes on.
/// 3. A connection that ended stays ended: the link is down, every
///    use is refused `connection` with the reason, and no use dials
///    again. The link goes down when the router's read loop ends —
///    the socket closed, however it closed — and when a send finds the
///    connection or its router gone first.
/// 4. [`reset`](Link::reset) puts the link back to not yet dialled, and
///    the next use dials. It is the page's one way to dial again.
///
/// The router runs on a task spawned on the current tokio runtime, so
/// [`handle`](Link::handle) is called from inside one.
pub struct Link {
    address: String,
    key: String,
    inner: Arc<Mutex<Inner>>,
}

impl Link {
    /// A link to the daemon at `address`, presenting `key`, not yet
    /// dialled. See [`dial`] for what an address may be.
    pub fn new(address: impl Into<String>, key: impl Into<String>) -> Link {
        Link { address: address.into(), key: key.into(), inner: Arc::new(Mutex::new(Inner { state: State::Idle, dials: 0 })) }
    }

    /// The handle: the connection that is up; or the one dialled now,
    /// when nothing has been dialled since the link was made or reset;
    /// or, when the connection ended or the dial failed, the refusal
    /// that says so. The lock is held through the dial, so uses that
    /// race the first wait for its one connection rather than making
    /// their own.
    pub async fn handle(&self) -> Result<Handle, Refusal> {
        let mut inner = self.inner.lock().await;
        match &inner.state {
            State::Up { handle, .. } => return Ok(handle.clone()),
            State::Down(message) => return Err(Refusal::Connection(message.clone())),
            State::Idle => {}
        }
        match dial(&self.address, &self.key).await {
            Ok((handle, run)) => {
                inner.dials += 1;
                let dialled = inner.dials;
                inner.state = State::Up { handle: handle.clone(), dialled };
                let shared = Arc::clone(&self.inner);
                tokio::spawn(async move {
                    run.await;
                    let mut inner = shared.lock().await;
                    if matches!(&inner.state, State::Up { dialled: up, .. } if *up == dialled) {
                        inner.state = State::Down(ENDED.to_string());
                    }
                });
                Ok(handle)
            }
            Err(error) => {
                let message = error.to_string();
                inner.state = State::Down(message.clone());
                Err(Refusal::Connection(message))
            }
        }
    }

    /// What a send came back with, as a refusal. A send that found the
    /// connection or its router gone takes the link down with its
    /// reason; a scope that is not open says nothing about the
    /// connection and leaves the link as it is.
    pub async fn sent<T>(&self, result: Result<T, SendError>) -> Result<T, Refusal> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let refusal = Refusal::from(error);
                if let Refusal::Connection(message) = &refusal {
                    let mut inner = self.inner.lock().await;
                    if matches!(inner.state, State::Up { .. }) {
                        inner.state = State::Down(message.clone());
                    }
                }
                Err(refusal)
            }
        }
    }

    /// How the link stands, without dialling: `dialling` while a use
    /// holds the lock, which only a dial does for long.
    pub fn status(&self) -> WireStatus {
        match self.inner.try_lock() {
            Err(_) => WireStatus::Dialling,
            Ok(inner) => match &inner.state {
                State::Idle => WireStatus::Idle,
                State::Up { .. } => WireStatus::Connected,
                State::Down(message) => WireStatus::Failed { message: message.clone() },
            },
        }
    }

    /// Put the link back to not yet dialled, whatever it was: the next
    /// use dials. A connection that was up is dropped with its last
    /// handle.
    pub async fn reset(&self) {
        self.inner.lock().await.state = State::Idle;
    }
}
