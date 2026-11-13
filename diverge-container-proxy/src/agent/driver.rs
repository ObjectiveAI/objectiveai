//! The queue's driver: one task that owns the queue, the loop's
//! state, the gate, and every decision about what the agent's server
//! is asked.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::enqueue::request::{Item, Message};
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use tokio::sync::{mpsc, oneshot};

use super::{Batch, Cmd, Fate, Outcome, run};
use crate::encode::encoded;
use crate::proxy::Proxy;

/// The batches handed to the loop in flight — one `/enqueue` on the
/// wire — and not yet answered.
struct Delivering {
    /// The batches on the wire, in the order the queue held them.
    batches: Vec<Batch>,
    /// The `/enqueue` that carries them: its outcome, to come.
    outcome: oneshot::Receiver<Outcome>,
}

/// What a run was started on, which is what may join it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Chat messages: a loop, which chat messages join by `/enqueue`.
    Chat,
    /// One structured decision: nothing joins it, chat or decision.
    Decision,
}

/// Whether a run is open.
enum Run {
    /// No run. The head of the queue starts one.
    Idle,
    /// `/run` is on the wire, with these batches as its content — the
    /// chat batches joined, or the one decision; their fates follow
    /// its answer.
    Starting(Vec<Batch>, Mode),
    /// A run is open, and its chunks are being relayed.
    Active(Mode),
}

/// A feed the server sent and the driver has not put on the wire.
struct Feed {
    values: Values,
    reply: oneshot::Sender<Result<(), Error>>,
}

/// The feed on the wire: the program's answer to come, and where the
/// server's channel hears it.
struct Feeding {
    outcome: oneshot::Receiver<Result<(), Error>>,
    reply: oneshot::Sender<Result<(), Error>>,
}

/// What the driver owns. Invariants, kept by there being one task:
///
/// 1. Only the driver reads or writes any of this.
/// 2. A batch is in exactly one place until its fate is sent: the
///    queue, the batches being delivered, or a starting run's. A
///    batch is never split, and its messages stay in its order.
/// 3. The queue is in the order of the batches' indexes, always. A
///    batch reaches the driver with the index its enqueue carried,
///    and joins the back of the queue only once every index before
///    its own has: one that arrives early waits in `arrived`. The
///    batches that come back go to the front, in the order they held,
///    ahead of everything queued after them.
/// 4. What is sent is the head of the queue, by what the head is. A
///    run of chat batches at the head, as far as the first structured
///    decision, is sent whole and joined — in a `/run`, or in an
///    `/enqueue` to a chat run — every batch's messages in order,
///    which the agent's server takes as one user turn. A structured
///    decision at the head is sent alone, in a `/run` of its own, and
///    only to a program with no run open: it waits for the run in
///    flight to end, whatever that run was started on.
/// 5. While a structured decision's run is open nothing is sent:
///    no chat batch joins it and no decision follows it until it has
///    ended. So a decision is handed over after everything before it
///    has finished, and finishes before anything after it starts.
/// 6. At most one call carrying messages is on the wire — a `/run` or
///    an `/enqueue` — so the agent's server receives every batch in
///    order, and batches that come back are never overtaken by a
///    newer one.
/// 7. A batch's fate is the answer to the call that carried it, and
///    the batches in one call share it: `Delivered` on the `/run`'s
///    `2xx` or the `/enqueue`'s `204`, the error on a refusal. Nothing
///    is withdrawn, and no fate waits for a turn.
/// 8. The driver never awaits the agent's server: every call is a
///    task of its own, reporting back on a channel.
/// 9. The gate is the driver's: a run is launched, or a batch
///    delivered, only while `fed`. It is shut at the begin, whatever
///    the program declared. It opens in the step that reads a feed's
///    success — the same step that answers the server's `Fed` — save
///    the deploy's feed, which leaves it shut; and it shuts in the
///    step that reads the loop's end — the same step that writes
///    `Inactive` — so the server never hears `Fed` for a gate that
///    then shuts behind it.
/// 10. What the driver writes on the begin scope's main stream is
///    `Inactive`, with the rotating secrets collected, and — when the
///    collect failed — the one `Error` that ends the run, after which
///    the begin loop finishes the scope. The `Active` and the chunks
///    are [`run::relay`]'s.
struct Driver {
    queue: VecDeque<Batch>,
    /// Batches that reached the driver ahead of an index not yet
    /// seen, by index: invariant 3.
    arrived: BTreeMap<u64, Batch>,
    /// The index the next batch to join the queue carries: `0` at the
    /// begin, one more for each that has joined.
    next: u64,
    delivering: Option<Delivering>,
    run: Run,
    /// The loop in flight closed under an `/enqueue`, or would not
    /// take one: nothing more is sent to it, and the next loop starts
    /// on the queue once it ends.
    closed: bool,
    /// The deploy's feed is still to come: the program registered
    /// something, so the provider feeds once before the run's id is
    /// out — before any message can be enqueued — and that feed
    /// applies the values and leaves the gate shut.
    registering: bool,
    /// The gate: whether a feed has come since the begin, or since the
    /// last loop ended. It is the go: nothing is launched or sent
    /// while it is shut, and a batch that reaches it shut waits for
    /// the next feed.
    fed: bool,
    /// Feeds waiting behind the one on the wire, in order.
    feeds: VecDeque<Feed>,
}

/// Start the driver for an agent container that has begun, and hand
/// back the channel the server's channels speak to it on.
/// `registering` is whether the program registered anything — a
/// dependency or a secret — which is when the provider feeds it at
/// the deploy.
pub fn driver(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, registering: bool) -> mpsc::UnboundedSender<Cmd> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(drive(proxy, scope, registering, receiver));
    sender
}

/// The loop: one event, then the one post-step where runs are
/// launched, batches delivered and feeds put on the wire.
///
/// The rules, by event:
///
/// 1. An enqueue's batch joins the back of the queue when its index
///    is the next, and every batch waiting on it after it; one ahead
///    of its turn waits.
/// 2. The `/run`'s answer: a `2xx` delivers every batch it carried
///    and starts the relay; anything else answers every one with the
///    error.
/// 3. The `/enqueue`'s outcome: `Delivered` delivers every batch it
///    carried. `Returned` — the run closed under it, or the agent's
///    server could not take it — puts the batches back at the front
///    of the queue, in the order they held, and closes the loop in
///    flight — where one still is — to further batches, so the next
///    `/run` starts on them. `Refused` answers every batch with the
///    error.
/// 4. The loop's end makes the run idle and open to batches again,
///    and shuts the gate, in the step that writes `Inactive`.
/// 5. A feed goes on the wire behind any before it. Its success opens
///    the gate — every feed's, an empty one's too, save the deploy's
///    while `registering`, which only clears that — and the post-step
///    then launches `/run` on the whole queue, joined, when there is
///    one: the gate is the go, and a queue that is empty then is
///    launched by the first batch to arrive behind the open gate. A
///    refusal leaves the gate as it was.
///
/// The post-step runs after EVERY event. While an `/enqueue` is on
/// the wire it sends nothing; otherwise, behind an open gate and with
/// a queue, it sends the head of it as invariants 4 and 5 state: with
/// no run open, a `/run` — on the chat batches at the head, joined,
/// or on the one structured decision there; with a chat run open and
/// not closed, one `/enqueue` of the chat batches at the head, where
/// there are any; and nothing while a decision's run is open. That is what closes the race between a batch
/// and the loop's end: a loop ending before its batch's `409` leaves
/// the batch on the wire — and nothing launched — until the outcome
/// puts it back at the front of the queue, and the next post-step
/// launches once a feed has opened the gate; a
/// `409` before the loop's end closes the loop, and the drain is
/// launched once the end has come and a feed has opened the gate.
/// The command channel closing is the connection gone — every
/// channel that held a sender is over — and the driver ends with it;
/// the calls in flight end with the process.
async fn drive(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, registering: bool, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let mut driver = Driver {
        queue: VecDeque::new(),
        arrived: BTreeMap::new(),
        next: 0,
        delivering: None,
        run: Run::Idle,
        closed: false,
        registering,
        fed: false,
        feeds: VecDeque::new(),
    };
    let mut starting: Option<oneshot::Receiver<Result<reqwest::Response, Error>>> = None;
    let mut ended: Option<oneshot::Receiver<Result<Values, Error>>> = None;
    let mut feeding: Option<Feeding> = None;
    // The last loop's `Inactive` or `Error`, on its way: the next loop's
    // relay waits for it, so the main stream keeps its order.
    let mut word: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            command = commands.recv() => match command {
                None => break,
                Some(Cmd::Enqueue(index, batch)) => {
                    // Rule 1: held until its turn, then queued with
                    // every batch that waited on it.
                    driver.arrived.insert(index, batch);
                    while let Some(batch) = driver.arrived.remove(&driver.next) {
                        driver.queue.push_back(batch);
                        driver.next += 1;
                    }
                }
                Some(Cmd::Feed { values, reply }) => driver.feeds.push_back(Feed { values, reply }),
            },
            started = wait(starting.as_mut()), if starting.is_some() => {
                starting = None;
                let (batches, mode) = match std::mem::replace(&mut driver.run, Run::Idle) {
                    Run::Starting(batches, mode) => (batches, mode),
                    Run::Idle | Run::Active(_) => (Vec::new(), Mode::Chat),
                };
                match started {
                    Ok(Ok(response)) => {
                        for batch in batches {
                            let _ = batch.fate.send(Fate::Delivered);
                        }
                        ended = Some(run::relay(Arc::clone(&proxy), response, Arc::clone(&scope), word.take()));
                        driver.run = Run::Active(mode);
                    }
                    Ok(Err(error)) => {
                        for batch in batches {
                            let _ = batch.fate.send(Fate::Error(error.clone()));
                        }
                    }
                    Err(_) => {
                        for batch in batches {
                            let _ = batch.fate.send(Fate::Error(died()));
                        }
                    }
                }
            },
            collected = wait(ended.as_mut()), if ended.is_some() => {
                ended = None;
                driver.run = Run::Idle;
                driver.closed = false;
                // The gate shuts with the word, in one step: nothing
                // launches until the next feed opens it.
                driver.fed = false;
                // Sent on a task of its own: the main stream is paced
                // by the daemon's reading (see `credit`), and the
                // driver waiting on it would hold every feed and
                // enqueue behind it. Order holds: the relay's last
                // frame went before `ended` fired, and the next loop's
                // relay waits for this one (`word`).
                let (frame, ends) = match collected {
                    Ok(Ok(values)) => (response::Frame::Inactive(values), false),
                    Ok(Err(error)) => (response::Frame::Error(error), true),
                    Err(_) => (response::Frame::Error(uncollected()), true),
                };
                let (scope, proxy) = (Arc::clone(&scope), Arc::clone(&proxy));
                word = Some(tokio::spawn(async move {
                    send(&scope, &frame).await;
                    if ends {
                        proxy.end();
                    }
                }));
            },
            outcome = wait(driver.delivering.as_mut().map(|delivering| &mut delivering.outcome)), if driver.delivering.is_some() => {
                let Some(Delivering { batches, .. }) = driver.delivering.take() else {
                    continue;
                };
                match outcome.unwrap_or(Outcome::Returned) {
                    Outcome::Delivered => {
                        for batch in batches {
                            let _ = batch.fate.send(Fate::Delivered);
                        }
                    }
                    Outcome::Refused(error) => {
                        for batch in batches {
                            let _ = batch.fate.send(Fate::Error(error.clone()));
                        }
                    }
                    Outcome::Returned => {
                        // Nothing more for this loop: the next starts
                        // on the batches. A loop that has ended already
                        // is not closed: the flag is the loop in flight's,
                        // and would otherwise shut the next one.
                        driver.closed = matches!(driver.run, Run::Active(_));
                        // Back at the front, in the order they held:
                        // the last first, so the first ends up first,
                        // ahead of anything that came after them.
                        for batch in batches.into_iter().rev() {
                            driver.queue.push_front(batch);
                        }
                    }
                }
            },
            applied = wait(feeding.as_mut().map(|feeding| &mut feeding.outcome)), if feeding.is_some() => {
                let Some(Feeding { reply, .. }) = feeding.take() else {
                    continue;
                };
                // The gate opens with the word, in one step — save at
                // the deploy's feed (rule 5).
                match applied {
                    Ok(Ok(())) => {
                        if driver.registering {
                            driver.registering = false;
                        } else {
                            driver.fed = true;
                        }
                        let _ = reply.send(Ok(()));
                    }
                    Ok(Err(error)) => {
                        let _ = reply.send(Err(error));
                    }
                    Err(_) => {
                        let _ = reply.send(Err(unfed()));
                    }
                }
            },
        }

        // The post-step: the only place a run is launched, a batch
        // delivered, or a feed put on the wire.
        if driver.delivering.is_none() && driver.fed && !driver.queue.is_empty() {
            match driver.run {
                // Invariant 4: no run open, so the head starts one —
                // the one decision there, alone, or the chat batches
                // as far as the first decision, joined.
                Run::Idle => {
                    let decision = match driver.queue.front().map(|batch| &batch.item) {
                        Some(Item::StructuredDecisions(decision)) => Some(decision.clone()),
                        _ => None,
                    };
                    match decision {
                        Some(decision) => {
                            let batches: Vec<Batch> = driver.queue.pop_front().into_iter().collect();
                            starting = Some(run::start(Arc::clone(&proxy), Item::StructuredDecisions(decision)));
                            driver.run = Run::Starting(batches, Mode::Decision);
                        }
                        None => {
                            let batches = chats(&mut driver.queue);
                            starting = Some(run::start(Arc::clone(&proxy), Item::Chat { messages: messages(&batches) }));
                            driver.run = Run::Starting(batches, Mode::Chat);
                        }
                    }
                }
                // Invariant 4: a chat run takes the chat batches at
                // the head, and a decision there waits for its end.
                Run::Active(Mode::Chat) if !driver.closed => {
                    let batches = chats(&mut driver.queue);
                    if !batches.is_empty() {
                        driver.delivering = Some(Delivering {
                            outcome: super::deliver(Arc::clone(&proxy), messages(&batches)),
                            batches,
                        });
                    }
                }
                // Invariant 5: a decision's run takes nothing.
                Run::Starting(..) | Run::Active(_) => {}
            }
        }
        if feeding.is_none() {
            if let Some(Feed { values, reply }) = driver.feeds.pop_front() {
                feeding = Some(Feeding {
                    outcome: run::feed(Arc::clone(&proxy), values),
                    reply,
                });
            }
        }
    }
}

/// The chat batches at the head of the queue, taken: every one as far
/// as the first structured decision, in order. None where the head is
/// a decision.
fn chats(queue: &mut VecDeque<Batch>) -> Vec<Batch> {
    let mut taken = Vec::new();
    while queue.front().is_some_and(|batch| !batch.is_decision()) {
        taken.extend(queue.pop_front());
    }
    taken
}

/// Chat batches as the loop is asked them: every message of every
/// batch, under its key, the batches in the order the queue held them
/// and each batch's messages in its own. A structured decision among
/// them has no messages and adds none; [`chats`] hands this none.
fn messages(batches: &[Batch]) -> Vec<Message> {
    batches
        .iter()
        .flat_map(|batch| match &batch.item {
            Item::Chat { messages } => messages.clone(),
            Item::StructuredDecisions(_) => Vec::new(),
        })
        .collect()
}

/// The slot's answer, or forever when there is no slot: a branch
/// that is disabled is never polled, and this is what it would poll.
async fn wait<T>(slot: Option<&mut oneshot::Receiver<T>>) -> Result<T, oneshot::error::RecvError> {
    match slot {
        Some(receiver) => receiver.await,
        None => std::future::pending().await,
    }
}

/// One frame on the begin scope's main stream, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    if let Some(payload) = encoded(frame) {
        scope.send_response(&payload).await;
    }
}

/// The task that was to start the run ended without saying how.
fn died() -> Error {
    Error(serde_json::json!({
        "kind": "agent",
        "error": "the call that was to start the run died",
    }))
}

/// The task that was to collect the secrets ended without saying how.
fn uncollected() -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": "the task that was to collect the secrets died",
    }))
}

/// The task that was to feed the secrets ended without saying how.
fn unfed() -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": "the task that was to feed the secrets died",
    }))
}
