//! The one connection a native side holds for its page.

use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use super::{Redial, Refusal, WireStatus};
use crate::wire::client::dial::dial;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::handshake::Authenticator;

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

/// What the link and its redial share.
struct Shared {
    address: String,
    issuer: String,
    authenticator: Arc<dyn Authenticator>,
    redial: Redial,
    inner: Mutex<Inner>,
    /// Told when the link goes down from up, and at every reset: what
    /// the redial waits on.
    changed: Notify,
}

/// A daemon's address, the issuer and the authenticator its handshake
/// is made with, and the one connection to it, as a native side holds
/// it for a page that speaks through the proxy. Every dial presents a
/// token the authenticator gives it then, so a redial never presents
/// a token that expired.
///
/// The rules, a socket client's, with the [`Redial`] the link is made
/// with:
///
/// 1. A use while nothing has been dialled — since the link was made or
///    reset — dials, holding the lock through the dial, so uses that
///    race wait for its one connection rather than making their own.
/// 2. A connection that is up is the one every send goes on.
/// 3. A connection that ended is down: every use is refused
///    `connection` with the reason, and no use dials. The link goes
///    down when the router's read loop ends — the socket closed,
///    however it closed — and when a send finds the connection or its
///    router gone first.
/// 4. With [`Redial::Never`], a link that is down stays down;
///    [`reset`](Link::reset) puts it back to not yet dialled, and the
///    next use dials.
/// 5. With [`Redial::Every`], the [`redial`](Link::redial) task — which
///    the native side spawns beside the link — dials at once, and
///    dials again that long after every failed dial and every
///    connection that ended, for as long as the task runs, without
///    backing off; a [`reset`](Link::reset) dials at once, a
///    connection that was up dropped first. While it dials it holds the
///    lock, as a use does.
///
/// The router runs on a task spawned on the current tokio runtime, so
/// [`handle`](Link::handle) and [`redial`](Link::redial) run inside
/// one.
pub struct Link(Arc<Shared>);

impl Link {
    /// A link to the daemon at `address`, its handshake made from
    /// `issuer` through `authenticator`, not yet dialled, redialling as
    /// `redial` says. See [`dial`] for what an address may be.
    pub fn new(address: impl Into<String>, issuer: impl Into<String>, authenticator: Arc<dyn Authenticator>, redial: Redial) -> Link {
        Link(Arc::new(Shared {
            address: address.into(),
            issuer: issuer.into(),
            authenticator,
            redial,
            inner: Mutex::new(Inner { state: State::Idle, dials: 0 }),
            changed: Notify::new(),
        }))
    }

    /// The redial, rule 5: a task for the native side to spawn beside
    /// the link, which runs for as long as it is not dropped. With
    /// [`Redial::Never`] it ends at once, having done nothing.
    pub fn redial(&self) -> impl Future<Output = ()> + Send + 'static {
        let shared = Arc::clone(&self.0);
        async move {
            let Redial::Every(pause) = shared.redial else {
                return;
            };
            loop {
                {
                    let mut inner = shared.inner.lock().await;
                    if !matches!(inner.state, State::Up { .. }) {
                        let _ = dial_locked(&shared, &mut inner).await;
                    }
                }
                // Up: wait for it to go down, or to be reset.
                let reset = loop {
                    {
                        let inner = shared.inner.lock().await;
                        match inner.state {
                            State::Up { .. } => {}
                            State::Idle => break true,
                            State::Down(_) => break false,
                        }
                    }
                    shared.changed.notified().await;
                };
                if !reset {
                    // Down: the pause, which a reset cuts short.
                    tokio::select! {
                        _ = tokio::time::sleep(pause) => {}
                        _ = shared.changed.notified() => {}
                    }
                }
            }
        }
    }

    /// The handle: the connection that is up; or the one dialled now,
    /// when nothing has been dialled since the link was made or reset;
    /// or, when the connection ended or the dial failed, the refusal
    /// that says so.
    pub async fn handle(&self) -> Result<Handle, Refusal> {
        let mut inner = self.0.inner.lock().await;
        match &inner.state {
            State::Up { handle, .. } => Ok(handle.clone()),
            State::Down(message) => Err(Refusal::Connection(message.clone())),
            State::Idle => dial_locked(&self.0, &mut inner).await,
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
                    let mut inner = self.0.inner.lock().await;
                    if matches!(inner.state, State::Up { .. }) {
                        inner.state = State::Down(message.clone());
                        self.0.changed.notify_one();
                    }
                }
                Err(refusal)
            }
        }
    }

    /// How the link stands, without dialling: `dialling` while the lock
    /// is held, which only a dial does for long.
    pub fn status(&self) -> WireStatus {
        match self.0.inner.try_lock() {
            Err(_) => WireStatus::Dialling,
            Ok(inner) => match &inner.state {
                State::Idle => WireStatus::Idle,
                State::Up { .. } => WireStatus::Connected,
                State::Down(message) => WireStatus::Failed { message: message.clone() },
            },
        }
    }

    /// Put the link back to not yet dialled, whatever it was: the next
    /// use dials, or — redialling — the redial dials at once. A
    /// connection that was up is dropped with its last handle.
    pub async fn reset(&self) {
        self.0.inner.lock().await.state = State::Idle;
        self.0.changed.notify_one();
    }
}

/// Dial with the lock held: up with the new connection, whose end takes
/// the link down and tells the redial; or down with the reason, which
/// is the refusal.
async fn dial_locked(shared: &Arc<Shared>, inner: &mut Inner) -> Result<Handle, Refusal> {
    match dial(&shared.address, &shared.issuer, shared.authenticator.as_ref()).await {
        Ok((handle, run, _)) => {
            inner.dials += 1;
            let dialled = inner.dials;
            inner.state = State::Up { handle: handle.clone(), dialled };
            let watched = Arc::clone(shared);
            tokio::spawn(async move {
                run.await;
                let mut inner = watched.inner.lock().await;
                if matches!(&inner.state, State::Up { dialled: up, .. } if *up == dialled) {
                    inner.state = State::Down(ENDED.to_string());
                    watched.changed.notify_one();
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
