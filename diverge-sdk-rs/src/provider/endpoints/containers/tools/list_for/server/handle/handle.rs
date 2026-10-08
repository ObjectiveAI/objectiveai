//! Listing the tool containers an identity runs, and keeps running,
//! from a scope and the directory.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::task::JoinSet;

use crate::provider::endpoints::containers::server::encoded::encoded;
use crate::provider::endpoints::containers::server::run::send;
use crate::provider::endpoints::containers::tools::list_for::client::{channel_request, request};
use crate::provider::endpoints::containers::tools::list_for::server::response;
use crate::provider::endpoints::containers::tools::run::server::channel_request as run_channel_request;
use crate::provider::server::directory::{Change, Directory, Running};
use crate::shared::containers::authorize;
use crate::wire::decode::Decode as _;
use crate::wire::frame::client::ClientFrame;
use crate::wire::server::answer::{Answer, answer};
use crate::wire::server::scope_handle::ScopeHandle;

/// Send the lister every tool container the identity runs whose
/// runner allows it, each as its runner answers, the word that the
/// listing is whole, and every change after, until the lister stops.
///
/// In order:
///
/// 1. The directory's changes subscribed to, before anything is
///    read, so nothing that happens meanwhile is missed; then every
///    tool container the identity runs now, found in the
///    [`Directory`] by its runner.
/// 2. Each runner asked, on its RUN scope, at once and all together:
///    an [`AuthorizeList`](authorize::request::AuthorizeList) carrying
///    the lister's address — this connection's peer, attested — and
///    the identity this connection was authorized under, attested
///    too. One frame answers each.
/// 3. `Authorized`: the container sent as added the moment the answer
///    lands, without waiting on any other runner — unless its run
///    ended meanwhile, when nothing is sent. `Denied`, a finish with
///    nothing, a runner that is gone: nothing sent.
/// 4. Once every container found in step 1 has been answered for or
///    its runner has gone, the one `Listed`, sent at once when none
///    was found.
/// 5. From the subscription, for the scope's life: a tool container
///    the identity starts is asked about as in step 2 and sent as
///    added as in step 3; one whose run ends is sent as removed when
///    it was sent as added, else nothing. A subscription that fell
///    behind is reconciled against the directory: containers not yet
///    asked about are asked, and containers added that no longer run
///    are sent as removed.
/// 6. The finish, when the lister's [`Stop`](channel_request::Frame::Stop)
///    arrives or the lister is gone. Asks still in flight are let go;
///    their channels end with the runs they are on.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    address: IpAddr,
    directory: Arc<Directory>,
) {
    let scope = Arc::new(scope);
    let request::Identity::Unbrokered { identity: tenant } = request.0;
    let lister = authorize::request::AuthorizeList {
        address,
        identity: client_identity.to_string(),
    };
    let mut changes = directory.subscribe();
    let mut listing = Listing {
        scope: Arc::clone(&scope),
        lister,
        asks: JoinSet::new(),
        asked: HashSet::new(),
        sent: HashSet::new(),
        gone: HashSet::new(),
        opening: 0,
        listed: false,
    };
    for running in directory.running_under(&tenant).await {
        listing.ask(running, true);
    }
    listing.whole_if_opened().await;
    loop {
        tokio::select! {
            concluded = listing.asks.join_next(), if !listing.asks.is_empty() => {
                if let Some(Ok((id, opening, verdict))) = concluded {
                    listing.concluded(id, opening, verdict).await;
                }
            }
            change = changes.recv() => match change {
                Ok(Change::Started(running)) => {
                    if running.runner.as_ref() == tenant {
                        listing.ask(running, false);
                    }
                }
                Ok(Change::Ended { id }) => listing.ended(&id).await,
                Err(broadcast::error::RecvError::Lagged(_)) => listing.reconcile(&directory, &tenant).await,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Stop) => break,
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
        }
    }
    drop(listing);
    scope.send_response_finish().await;
}

/// One listing as it runs: who is asking, what has been asked, what
/// has been sent.
struct Listing {
    /// The scope the listing is sent on.
    scope: Arc<ScopeHandle>,
    /// The lister, as every runner is told of it.
    lister: authorize::request::AuthorizeList,
    /// Every ask in flight: the container's id, whether it was found
    /// at the opening, and the runner's verdict.
    asks: JoinSet<(String, bool, bool)>,
    /// Every container asked about, so none is asked twice.
    asked: HashSet<String>,
    /// Every container sent as added and not since removed.
    sent: HashSet<String>,
    /// Every container whose run ended while its ask was in flight.
    gone: HashSet<String>,
    /// How many containers found at the opening are not yet
    /// concluded.
    opening: usize,
    /// Whether `Listed` has been sent.
    listed: bool,
}

impl Listing {
    /// Ask the runner of `running`, on a task of its own; `opening`
    /// says whether the container was there when the scope opened.
    fn ask(&mut self, running: Running, opening: bool) {
        if !self.asked.insert(running.id.clone()) {
            return;
        }
        if opening {
            self.opening += 1;
        }
        let lister = self.lister.clone();
        self.asks.spawn(async move {
            let verdict = authorized(&running, lister).await;
            (running.id, opening, verdict)
        });
    }

    /// One ask concluded: the container sent as added when allowed
    /// and still running, and `Listed` sent when it was the last of
    /// the opening's.
    async fn concluded(&mut self, id: String, opening: bool, verdict: bool) {
        if verdict && !self.gone.remove(&id) {
            send(&self.scope, encoded(&response::Frame::Added(response::Container { id: id.clone() }))).await;
            self.sent.insert(id);
        } else {
            self.gone.remove(&id);
        }
        if opening {
            self.opening -= 1;
            self.whole_if_opened().await;
        }
    }

    /// `Listed`, once, when nothing found at the opening is left to
    /// conclude.
    async fn whole_if_opened(&mut self) {
        if self.opening == 0 && !self.listed {
            self.listed = true;
            send(&self.scope, encoded(&response::Frame::Listed)).await;
        }
    }

    /// The container's run ended: sent as removed when it was sent as
    /// added; a yes still to come for it is not sent.
    async fn ended(&mut self, id: &str) {
        if self.sent.remove(id) {
            send(&self.scope, encoded(&response::Frame::Removed(response::Container { id: id.to_string() }))).await;
        } else if self.asked.contains(id) {
            self.gone.insert(id.to_string());
        }
    }

    /// The subscription fell behind: the directory read again, every
    /// container not asked about asked, and every container sent
    /// that no longer runs sent as removed.
    async fn reconcile(&mut self, directory: &Directory, tenant: &str) {
        let running = directory.running_under(tenant).await;
        let ids: HashSet<String> = running.iter().map(|running| running.id.clone()).collect();
        for running in running {
            self.ask(running, false);
        }
        let removed: Vec<String> = self.sent.iter().filter(|id| !ids.contains(*id)).cloned().collect();
        for id in removed {
            self.ended(&id).await;
        }
    }
}

/// Ask the runner, on its scope, and read its one answer. The channel
/// is read to its finish, so its number comes back to the run.
async fn authorized(running: &Running, lister: authorize::request::AuthorizeList) -> bool {
    let Some(payload) = encoded(&run_channel_request::Frame::AuthorizeList(lister)) else {
        return false;
    };
    let mut channel = running.scope.send_channel_request(&payload).await;
    let mut verdict = false;
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                if let Ok(authorize::response::Frame::Authorized) = authorize::response::Frame::decode(&payload) {
                    verdict = true;
                }
            }
            Some(Answer::Finish) => break,
            None => {}
        }
    }
    verdict
}
