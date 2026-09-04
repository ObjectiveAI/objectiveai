//! The laboratories a provider is running, reachable by id.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc, oneshot};

/// Where a laboratory's own MCP server listens, inside the container.
///
/// The provider's number, not the caller's — a laboratory's request
/// declares no ports, because the provider puts the server in the
/// container itself and therefore picks where it binds.
///
/// It lives here rather than in either handler because BOTH need it
/// and they must agree: the run deploys the container with this in its
/// ports, and every connector's MCP ask dials it. Two copies would be
/// two numbers the day one changed.
// TODO: settled when the laboratory server is.
pub(crate) const MCP_PORT: u16 = 14978;

/// Every laboratory this provider is running, keyed by the id its
/// runner was told.
///
/// The one piece of shared state on the server half, and it exists
/// because two endpoints meet at one container. A
/// [`connect`](crate::endpoints::laboratories::connect) names a run by
/// its [`Id`](crate::endpoints::laboratories::run::server::response::Id),
/// and a [`transfer`](crate::shared::container::transfer) names a
/// destination the same way — and the scopes doing the naming are
/// strangers to the scope that owns the container: different
/// connections, different callers, nothing in common but the id. So
/// something above every scope has to resolve one, and this is it.
///
/// # A concrete type, not a trait
///
/// Like [`ClientRegistry`](super::client_registry::ClientRegistry),
/// and for the same reason: there is nothing here for a provider to
/// implement. Resolving an id is this crate's bookkeeping — the run
/// handler inserts, the connect handler and a transfer look up, and
/// the run handler removes. A provider constructs one, hands it to
/// every laboratories handler, and never touches it otherwise.
///
/// # An id is a capability
///
/// It is minted unguessable, and holding one is what entitles a caller
/// to NAME the laboratory: a transfer into a container is authorized by
/// knowing its id and by nothing else, which is why the id must never
/// be guessable and why nothing here checks anything further. A
/// connect is the exception — naming the laboratory only starts the
/// conversation, and whether the connector may attach is the runner's
/// answer to the authorization the run handler relays for it.
///
/// # What an entry holds, and what it deliberately does not
///
/// The container, shared, and the run handler's ear. NOT the run's
/// [`ScopeHandle`](super::scope_handle::ScopeHandle): finishing a scope
/// consumes the handle, which is only provable when nothing else holds
/// a share — so nothing else is ever given one, and everything a
/// connector wants said on the run's scope is said by the run handler,
/// asked through the entry's own event channel.
#[derive(Debug, Default)]
pub struct Laboratories<C> {
    /// The entries, by id.
    ///
    /// A tokio [`Mutex`] like every lock in this crate, held across
    /// nothing longer than an insert or a clone out.
    inner: Mutex<HashMap<String, Laboratory<C>>>,
}

impl<C> Laboratories<C> {
    /// An empty registry.
    ///
    /// A provider makes one and hands it to every laboratories
    /// handler. There is nothing to configure, because everything an
    /// entry holds arrives with the run that inserts it.
    pub fn new() -> Self {
        Laboratories {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Put a laboratory in, under the id its runner is being told.
    ///
    /// The run handler's, once, after the deploy succeeds and before
    /// the id goes out — so by the time anyone can quote the id, the
    /// entry is here to find.
    pub(crate) async fn insert(&self, id: String, laboratory: Laboratory<C>) {
        self.inner.lock().await.insert(id, laboratory);
    }

    /// Take a laboratory out.
    ///
    /// The run handler's, on its way down, and FIRST on its way down:
    /// a connect that resolves an id after this has begun would be
    /// joining a container that is about to stop existing.
    pub(crate) async fn remove(&self, id: &str) {
        self.inner.lock().await.remove(id);
    }

    /// The container and the run handler's ear, if the id names a
    /// laboratory that is running.
    ///
    /// Clones out rather than borrowing, so the lock is gone before
    /// anything is done with either — and because what a caller does
    /// with them outlives any borrow this could give: a connect holds
    /// both for its whole life, and a transfer holds the container for
    /// as long as a file takes.
    pub(crate) async fn get(
        &self,
        id: &str,
    ) -> Option<(Arc<C>, mpsc::UnboundedSender<Event>)> {
        self.inner.lock().await.get(id).map(|laboratory| {
            (
                Arc::clone(&laboratory.container),
                laboratory.events.clone(),
            )
        })
    }
}

/// One running laboratory, as the registry holds it.
///
/// Built by the run handler, which is also the only thing that ever
/// takes it back out.
#[derive(Debug)]
pub(crate) struct Laboratory<C> {
    /// The container, shared with whoever resolves the id.
    ///
    /// A connect serves its connector against this, and a transfer
    /// writes into it. The run handler keeps its own share, so this
    /// existing does not keep the container alive past the run —
    /// stopping it is [`stop`](super::container::Container::stop),
    /// which takes `&self` and cares nothing for counts.
    pub(crate) container: Arc<C>,
    /// The run handler's ear.
    ///
    /// # It does two jobs, and the second is the load-bearing one
    ///
    /// The first is carrying [`Event`]s: everything a connector needs
    /// said or asked on the run's scope travels through here, because
    /// the scope itself is never shared.
    ///
    /// The second is being the run's pulse. The run handler owns the
    /// receiver, so when the run ends — stopped, caller gone, however —
    /// the receiver drops and every clone of this sees
    /// [`closed`](mpsc::UnboundedSender::closed) resolve. That is how a
    /// connect scope learns the thing it joined is over, without the
    /// run knowing who joined: a connection cannot outlive the thing it
    /// joined, and this is the mechanism that makes it so.
    pub(crate) events: mpsc::UnboundedSender<Event>,
}

/// Something a connect handler needs the run handler to do.
///
/// The run's scope is spoken on only by the run handler — see
/// [`Laboratory::events`] for why — so what a connector needs said
/// there arrives as one of these instead.
#[derive(Debug)]
pub(crate) enum Event {
    /// A connector is at the door: ask the runner.
    ///
    /// The run handler opens an
    /// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Frame::Authorize)
    /// channel on its scope, and the runner's answer comes back through
    /// `reply`: the nickname it chose, or [`None`] for a refusal.
    ///
    /// [`None`] is also what a reply that never comes collapses into —
    /// a dropped [`oneshot::Sender`] and a `None` read the same at the
    /// door, and they should: a runner that could not be asked has not
    /// said yes.
    Authorize {
        /// Where the connector's socket comes from, as the provider
        /// sees it. A signal, not an identity — the
        /// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Authorize)
        /// it is copied into says why.
        address: IpAddr,
        /// Whatever the connector offered, verbatim. Opaque here as
        /// everywhere: the runner is who reads it.
        authorization: String,
        /// Where the verdict goes.
        reply: oneshot::Sender<Option<String>>,
    },
    /// A connector that was attached is not any more.
    ///
    /// The run handler turns it into a
    /// [`Disconnected`](crate::endpoints::laboratories::run::server::response::Frame::Disconnected)
    /// on its scope. Sent on EVERY way out of an authorized connect —
    /// graceful or not — because the runner saw every arrival and is
    /// owed every departure.
    Disconnected {
        /// The name the runner gave this connector when it said yes.
        nickname: String,
    },
}
