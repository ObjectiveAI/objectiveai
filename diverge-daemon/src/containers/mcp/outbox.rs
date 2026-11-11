//! One merged notification stream's queue: unbounded while it is read,
//! its list changes coalesced.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use futures_util::stream;
use rmcp::model::ServerNotification;
use tokio::sync::{Mutex, Notify, watch};

use super::Notifications;

/// The queue every producer of one merged notification stream pushes
/// into — the set's watcher and every served tool's relay — and the
/// stream reads from. The rules:
///
/// 1. **Unbounded while read.** Everything pushed is queued until it is
///    delivered, and nothing is dropped but what coalesces.
/// 2. **A list change coalesces with any undelivered one.** A tool list
///    change pushed while one is queued and undelivered queues nothing:
///    the queued one is delivered after this change happened, so the
///    reader lists again after it, and that listing sees this change.
///    The same holds for a resource list change. The one kept holds its
///    earliest place in the queue.
/// 3. **A resource update coalesces by its URI.** An update for a URI
///    already queued and undelivered queues nothing: reading the
///    resource once covers both.
/// 4. **Everything else keeps its order**, delivered as it came:
///    progress, logging, anything a tool says of its own.
/// 5. **Delivery clears the mark.** A list change, or a URI's update,
///    taken off the queue lets the next one queue again.
/// 6. **The merged list changes are the daemon's own.** They are
///    delivered as bare `notifications/tools/list_changed` and
///    `notifications/resources/list_changed`, with no tool's
///    attestation: no one tool said them.
/// 7. **Closed when the reader goes.** The stream dropped closes the
///    outbox: every producer waiting on [`closed`](Self::closed) ends,
///    a push after it is nothing, and the queue goes with the last
///    holder.
pub struct Outbox {
    state: Mutex<State>,
    /// Wakes the reader when something is queued.
    ready: Notify,
    /// Whether the reader has gone: set from a drop, so a watch and not
    /// the lock.
    closed: watch::Sender<bool>,
}

/// What is queued, and what is pending in it.
struct State {
    queue: VecDeque<Queued>,
    /// A tool list change is queued and undelivered (rule 2).
    tools: bool,
    /// A resource list change is queued and undelivered (rule 2).
    resources: bool,
    /// The URIs with an update queued and undelivered (rule 3).
    updated: HashSet<String>,
}

/// One queued item.
enum Queued {
    /// The merged tool list changed.
    Tools,
    /// The merged resource list changed.
    Resources,
    /// A resource was updated: its URI, and the notification as its tool
    /// sent it, attested.
    Updated(String, ServerNotification),
    /// Anything else, as it came.
    Other(ServerNotification),
}

/// What a producer pushes.
pub enum Push {
    /// The merged tool list changed: the set did, or a served tool says
    /// its own did.
    Tools,
    /// The merged resource list changed, likewise.
    Resources,
    /// A served tool's notification, attested: a resource update is
    /// coalesced by its URI (rule 3), anything else queued as it is.
    Notification(ServerNotification),
}

impl Outbox {
    /// An empty outbox, and the stream that reads it.
    pub fn open() -> (Arc<Outbox>, Notifications) {
        let outbox = Arc::new(Outbox {
            state: Mutex::new(State { queue: VecDeque::new(), tools: false, resources: false, updated: HashSet::new() }),
            ready: Notify::new(),
            closed: watch::channel(false).0,
        });
        let reader = Reader(Arc::clone(&outbox));
        let notifications = Box::pin(stream::unfold(reader, |reader| async move {
            let notification = reader.0.next().await?;
            Some((Ok(notification), reader))
        }));
        (outbox, notifications)
    }

    /// Queue `push`, as rules 2 to 4 say; nothing once closed (rule 7).
    pub async fn push(&self, push: Push) {
        if *self.closed.borrow() {
            return;
        }
        {
            let mut state = self.state.lock().await;
            let queued = match push {
                Push::Tools if state.tools => return,
                Push::Tools => {
                    state.tools = true;
                    Queued::Tools
                }
                Push::Resources if state.resources => return,
                Push::Resources => {
                    state.resources = true;
                    Queued::Resources
                }
                Push::Notification(notification) => match &notification {
                    ServerNotification::ResourceUpdatedNotification(updated) => {
                        let uri = updated.params.uri.clone();
                        if !state.updated.insert(uri.clone()) {
                            return;
                        }
                        Queued::Updated(uri, notification)
                    }
                    _ => Queued::Other(notification),
                },
            };
            state.queue.push_back(queued);
        }
        self.ready.notify_one();
    }

    /// Resolves once the reader has gone (rule 7).
    pub async fn closed(&self) {
        let mut closed = self.closed.subscribe();
        let _ = closed.wait_for(|closed| *closed).await;
    }

    /// The next notification, waiting for one; `None` once closed.
    async fn next(&self) -> Option<ServerNotification> {
        loop {
            // Made before the queue is looked at, so a push between the
            // look and the wait still wakes it.
            let ready = self.ready.notified();
            if *self.closed.borrow() {
                return None;
            }
            {
                let mut state = self.state.lock().await;
                if let Some(queued) = state.queue.pop_front() {
                    // Rules 5 and 6.
                    return Some(match queued {
                        Queued::Tools => {
                            state.tools = false;
                            ServerNotification::ToolListChangedNotification(Default::default())
                        }
                        Queued::Resources => {
                            state.resources = false;
                            ServerNotification::ResourceListChangedNotification(Default::default())
                        }
                        Queued::Updated(uri, notification) => {
                            state.updated.remove(&uri);
                            notification
                        }
                        Queued::Other(notification) => notification,
                    });
                }
            }
            ready.await;
        }
    }
}

/// The stream's hold on its outbox: dropped with the stream, which
/// closes it (rule 7).
struct Reader(Arc<Outbox>);

impl Drop for Reader {
    fn drop(&mut self) {
        self.0.closed.send_replace(true);
        self.0.ready.notify_one();
    }
}
