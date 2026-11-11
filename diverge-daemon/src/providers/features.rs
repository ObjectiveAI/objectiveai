//! What the daemon holds open on a provider's connection, as its
//! record says: its announce, its accept, and its listen.

use std::net::IpAddr;
use std::pin::pin;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::accept::client::execute::Accepting;
use diverge_sdk::provider::endpoints::daemons::announce::client::{execute, request};
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::description::Description;
use diverge_sdk::wire::client::handle::Handle;
use futures_util::StreamExt as _;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::daemon::Daemon;
use crate::listening;

/// How a provider's connection was made, and the IP at its other end:
/// kept in [`Live`](crate::daemon::Live) from the attach to the
/// connection's end, whatever the provider's standing. What an edit of
/// an incoming record's `ips` judges the live connection by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The provider dialled the daemon, from this IP: the one its
    /// record's `ips` admitted at the handshake.
    Accepted(IpAddr),
    /// The daemon dialled the provider, and reached it at this IP. No
    /// record's `ips` speak of it.
    Dialled(IpAddr),
}

/// One connected provider's three scopes: what the daemon holds open
/// on the connection besides the work it asks of it. In
/// [`Live`](crate::daemon::Live) from the provider's standing as
/// connected — its version the daemon's — to the connection's end, and
/// never for a provider of another version, which is used for nothing.
/// The rules:
///
/// 1. **Three, each its own.** The provider's `daemons::announce`,
///    `daemons::accept` and `daemons::listen`: each is open or not by
///    one member of the provider's record, whatever the other two say.
/// 2. **One lock.** What is held is behind one lock per connection,
///    taken for the whole of a [`sync`], so two syncs of one provider
///    never interleave.
/// 3. **Gone with the connection.** Nothing here outlives the
///    connection: it is dropped from `Live` at the connection's end,
///    where every scope on it has ended with the socket, and dropping
///    what was held sends nothing.
#[derive(Debug)]
pub struct Features {
    /// The caller's handle on the provider: the connection the three
    /// are opened on.
    handle: Handle,
    /// What the provider answered its info: whether it relays messages
    /// at all.
    info: Arc<Info>,
    /// What is open now (rule 2).
    held: Mutex<Held>,
}

/// The three scopes, each held or not.
#[derive(Debug, Default)]
struct Held {
    /// The announce, with what it announces.
    announce: Option<Announced>,
    /// The accept.
    accept: Option<Accepting>,
    /// The listen.
    listen: Option<listening::Listen>,
}

/// The announce scope held open.
#[derive(Debug)]
struct Announced {
    /// The text it was opened with: what a sync compares the record's
    /// against.
    announcement: Description,
    /// Its cancel.
    cancel: execute::Cancel,
    /// The task reading its stream to the end: done once the provider
    /// has finished the scope, or the connection has gone.
    finished: JoinHandle<()>,
}

impl Features {
    /// Nothing held yet, on the connection `handle` writes, of a
    /// provider that answered `info`.
    pub fn new(handle: Handle, info: Arc<Info>) -> Features {
        Features { handle, info, held: Mutex::new(Held::default()) }
    }
}

/// Make what is open on the provider `identity`'s connection what its
/// record says: called once the provider is connected, by
/// [`attach`](super::attach), and after every edit of its record, by
/// [`providers edit`](crate::serve::providers::edit). The rules:
///
/// 1. **Connected only.** A provider with no connection, one still
///    being asked its info, and one of another version have no
///    [`Features`], and nothing is done: the attach syncs when it
///    connects.
/// 2. **One at a time.** The connection's lock is taken first and held
///    to the end (see [`Features`]), and the record is read under it:
///    of two syncs, the later reads the record no earlier than the
///    first, so what is open ends as the latest record says.
/// 3. **What is wanted.** Each of the three is an ENABLED record's
///    word, read from the [`ProviderBook`](super::ProviderBook): the
///    announce is wanted with the record's `announcement`, when it has
///    one; the accept when `accept_daemons`; the listen when
///    `messageable` and the provider's info holds out `messages` — a
///    provider that relays none is never asked to.
/// 4. **Opened or cancelled.** Wanted and not held, it is opened;
///    held and not wanted, it is cancelled; an announce held with
///    another text than the record's is cancelled, and opened again
///    with the record's. Held as wanted, nothing is sent at all. The
///    three are independent, and are done at once.
/// 5. **The finish before the next.** A provider holds one of each per
///    connection, claims it as it reads the request, refuses a second
///    as `AlreadyOpen` rather than queueing it, and frees it just
///    before it sends the scope's finish — see
///    [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
///    The cancel is one frame and the next request another, and the
///    provider could read the second before its handler has acted on
///    the first; so a scope cancelled is read to its finish before the
///    sync goes on, and nothing is opened in its place until then.
/// 6. **Nothing is retried.** An open the provider refuses, or that
///    could not be sent, is left not held, and the next sync — the
///    next edit, or the next connection — asks again; no clock does.
///    A scope the provider ends after it was opened — an announce's
///    `AlreadyOpen`, which arrives on its stream, among them — stays
///    held as it was asked, and is asked again only when the record's
///    word for it changes, or on the next connection.
/// 7. **The accept's sessions.** Cancelling the accept is all it takes
///    to end every daemon's session that came through it: the provider
///    ends every connection relayed on an accept scope as the scope
///    ends, each session here then reading its connector gone. The
///    daemon keeps no list of them.
/// 8. **Never closes.** Nothing here closes the provider's connection,
///    and nothing here ends a run.
pub async fn sync(daemon: &Arc<Daemon>, identity: &Identity) {
    // Rule 1.
    let Some(features) = daemon.live.features(identity).await else {
        return;
    };
    // Rule 2.
    let mut held = features.held.lock().await;
    let Held { announce, accept, listen } = &mut *held;
    // Rule 3.
    let announcement = daemon.providers.announcement(identity);
    let accepts = daemon.providers.accepts_daemons(identity);
    let listens = daemon.providers.messageable(identity) && features.info.capabilities.messages.is_some();
    // Rule 4: three scopes, none waiting on another.
    tokio::join!(
        sync_announce(&features.handle, announce, announcement),
        sync_accept(daemon, &features.handle, accept, accepts),
        sync_listen(daemon, identity, &features.handle, listen, listens),
    );
}

/// Rules 4 to 6 for the announce: `held` made `wanted`.
async fn sync_announce(handle: &Handle, held: &mut Option<Announced>, wanted: Option<Description>) {
    if held.as_ref().map(|announced| &announced.announcement) == wanted.as_ref() {
        return;
    }
    if let Some(announced) = held.take() {
        // Rule 5: cancelled, and its finish read, before the next.
        announced.cancel.cancel().await;
        let _ = announced.finished.await;
    }
    let Some(announcement) = wanted else {
        return;
    };
    // Rule 6: an announce that does not go out is not held.
    let Ok((stream, cancel)) = execute::execute(handle, &request::Frame { announcement: announcement.clone() }).await else {
        return;
    };
    let finished = tokio::spawn(async move {
        // Nothing arrives while the announce stands; a refusal is its
        // one item, and is not acted on (rule 6).
        let mut stream = pin!(stream);
        while stream.next().await.is_some() {}
    });
    *held = Some(Announced { announcement, cancel, finished });
}

/// Rules 4 to 7 for the accept: `held` made `wanted`.
async fn sync_accept(daemon: &Arc<Daemon>, handle: &Handle, held: &mut Option<Accepting>, wanted: bool) {
    match (held.take(), wanted) {
        (Some(accepting), true) => *held = Some(accepting),
        (Some(mut accepting), false) => {
            // Rules 5 and 7: cancelled, and its finish read. A cancel
            // that does not go out is a scope, or a connection, gone.
            let _ = accepting.cancel().await;
            accepting.wait().await;
        }
        (None, true) => *held = super::accept(daemon, handle).await,
        (None, false) => {}
    }
}

/// Rules 4 to 6 for the listen: `held` made `wanted`.
async fn sync_listen(daemon: &Arc<Daemon>, provider: &Identity, handle: &Handle, held: &mut Option<listening::Listen>, wanted: bool) {
    match (held.take(), wanted) {
        (Some(listen), true) => *held = Some(listen),
        // Rule 5: cancelled, and read to its finish.
        (Some(listen), false) => listen.end().await,
        (None, true) => *held = listening::listen(daemon, provider.clone(), handle).await,
        (None, false) => {}
    }
}
