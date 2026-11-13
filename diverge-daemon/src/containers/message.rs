//! A message to an agent: the one way into it.

use std::sync::Arc;

use std::collections::HashMap;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::containers::structured_decisions::{Decision, Question};
use diverge_sdk::shared::error::Error;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use rmcp::model::ContentBlock;
use tokio::sync::oneshot;

use super::{Activated, Message, StartError, activate, events, stop};
use crate::daemon::Daemon;
use crate::serve::reply;
use crate::store::AgentId;
use crate::store::agents::Agent;

/// What became of a message.
#[derive(Debug, Clone)]
pub enum Fate {
    /// The agent's program has it: the loop it started, or its own
    /// queue, taken at its next seam.
    Delivered,
    /// The container refused it, the run is gone, or no run could be
    /// started or activated for it.
    Error(Error),
}

/// One message admitted to its agent's outbox and not yet taken by the
/// agent's pump: what it is, who sent it, and where its fate goes.
pub struct Waiting {
    /// The agent's record, as the message's handler read it: the pump
    /// starts the run from the latest one waiting.
    pub agent: Agent,
    /// The key the daemon minted for it: what each of its user parts
    /// carries back.
    pub key: String,
    /// What it is: a chat message, or a structured decision.
    pub what: What,
    /// Who sent it: what its user parts are logged as.
    pub sender: Creator,
    /// Where its fate goes, once: always answered by the pump, and
    /// dropped unanswered only when the pump died, which the handler
    /// reports as an error.
    pub fate: oneshot::Sender<Fate>,
}

/// What is admitted to an agent's outbox: the two things an agent is
/// asked, in one order.
pub enum What {
    /// A chat message: its content, in order.
    Chat(Vec<ContentBlock>),
    /// A structured decision: its content, and its questions.
    Decision {
        /// The prompt's content, in order.
        content: Vec<ContentBlock>,
        /// The questions, in order.
        questions: Vec<Question>,
    },
}

/// One request the pump sends: the keys of what it carries, and where
/// their fates go — one of each for a decision, one for every message
/// of a chat batch.
struct Sent {
    keys: Vec<String>,
    fates: Vec<oneshot::Sender<Fate>>,
}

/// Admit the message to the agent's outbox, under a key the daemon
/// mints, with who sent it kept so its user parts are logged as
/// theirs, and hand back its fate to come: `Delivered` once the
/// agent's program has it, or the error — however long the run takes
/// to start, to be activated, and its loop to take the batch. A sender
/// dropped with no fate said is the pump dead, which the caller
/// reports as an error. Nothing takes a message back once admitted.
///
/// The order the messages are admitted in is the order the agent
/// takes them in: the caller admits in the order its connection sent
/// them — see [`Turn`](crate::serve::Turn) — and the outbox and its
/// [`pump`] keep it. The first message admitted where no pump runs
/// starts one, here, before anything is waited on, as rule 1 of
/// [`Live::admit`](crate::daemon::Live::admit) states.
pub async fn admit(daemon: &Arc<Daemon>, agent: Agent, what: What, sender: Creator) -> oneshot::Receiver<Fate> {
    let (fate, heard) = oneshot::channel();
    let id = agent.id;
    let key = uuid::Uuid::new_v4().to_string();
    if daemon.live.admit(id, Waiting { agent, key, what, sender, fate }).await {
        tokio::spawn(watched(Arc::clone(daemon), id));
    }
    heard
}

/// The agent's [`pump`], in a task of its own, watched: a pump that
/// ended by itself removed its own entry; one that died — it panicked,
/// or the runtime is shutting down — has its entry forgotten here,
/// with whatever waited in it, as rule 4 of
/// [`Live::admit`](crate::daemon::Live::admit) states, so no message
/// waits on a pump that is not there.
async fn watched(daemon: Arc<Daemon>, id: AgentId) {
    let pumping = tokio::spawn(pump(Arc::clone(&daemon), id));
    if pumping.await.is_err() {
        daemon.live.forget_outbox(id).await;
    }
}

/// The agent's pump: the one task that takes from the agent's outbox,
/// and the one sender of its messages. It sends everything waiting, in
/// the order it was admitted, to a run that is up and fed, and waits
/// for every answer before it looks again — so the feed is answered
/// before anything leaves, and nothing can overtake anything: the
/// order and the joining hold by construction. The rules, one turn of
/// the loop each time:
///
/// 1. **Look.** Nothing waiting ends the pump, its entry removed in
///    the same look: rule 3 of
///    [`Live::admit`](crate::daemon::Live::admit). Else the agent's
///    record is the latest waiting message's.
/// 2. **Get the run** — [`agent`](super::agent), which starts it when
///    it is not up. Nothing is taken from the outbox meanwhile, so
///    every message admitted while the run starts waits there, in
///    order: that is the cold start's buffer. A start that fails
///    takes everything waiting and answers each with the start's
///    error.
/// 3. **Hold, then activate.** The run's `pending` is entered — held
///    from here until the batch is answered, so the idle clock does
///    not run and an `inactive` that crosses the batch feeds again:
///    see [`inactive_agent`](super::inactive_agent) — and the run is
///    activated: [`activate_agent`](super::activate_agent), the feed
///    sent where the gate is shut and its answer waited for.
///    - A run the daemon is ending — its idle clock stopping it — is
///      not activated, and nothing is taken: the hold is let go, the
///      run waited for until it is gone —
///      [`AgentRun::gone`](super::AgentRun::gone) — and the pump
///      looks again, which gets a fresh run. A message is never lost
///      to an idle stop.
///    - An activation that fails takes everything waiting and answers
///      each with its error, kind `secrets`.
/// 4. **Take everything waiting**, only now — as late as there is —
///    in the order it was admitted. Each message is put on
///    the graph — one event from its sender to the agent, whatever
///    its fate, since the graph records what was sent and not what
///    landed — and kept in the run's messages, as rule 1 of
///    [`Message`] states; then the run is touched.
/// 5. **Group, and number.** What was taken becomes requests, in
///    order: every run of chat messages with no decision between them
///    is one request, a batch; every structured decision is a request
///    of its own. Each request takes the run's next `index` —
///    [`AgentRun::next_index`](super::AgentRun::next_index), one
///    counter for chat and decisions alike — which is the order the
///    container's proxy hands them over in, whatever order they reach
///    it in: see
///    [`enqueue::request::Request`].
/// 6. **Send them all, and wait for them all.** Every request goes
///    out at once, each an enqueue of its own, and each answer is
///    taken as it comes: `Delivered` is the fate of every message of
///    that request, the container's `Error` is theirs, and an enqueue
///    that failed on the way — the run ending under it, the provider
///    gone — is the daemon's error for them. A decision's answer comes
///    when the run it starts has begun, which is after every request
///    before it has finished: the proxy holds it until then. The run's
///    entries are marked or removed as rules 2 and 3 of [`Message`]
///    state, each fate is said as its request is answered, and the
///    hold is let go once every request is.
///    - A loop that ends as the batch crosses it shuts the gate on
///      the batch, and the run's own pump opens it again, since the
///      hold says a batch is pending. Where that failed — the run's
///      `unfed` told — the pump activates the run again itself, while
///      it waits. A feed that fails again leaves a batch no feed will
///      reach: the run is ended as failed —
///      [`fail_agent`](super::fail_agent), which takes the batch with
///      it — every message's fate is that error, kind `secrets`, and
///      what waits after starts the next run.
/// 7. **Look again.**
///
/// Every message taken is answered in the turn that took it: nothing
/// is put back, and nothing is held across a turn. The pump is no
/// task of any run's, so a run's end never ends it; a pump that dies
/// is [`watched`]'s.
async fn pump(daemon: Arc<Daemon>, id: AgentId) {
    while let Some(agent) = daemon.live.waiting(id).await {
        let run = match super::agent(&daemon, &agent).await {
            Ok(run) => run,
            Err(error) => {
                answer(daemon.live.take_waiting(id).await, &Fate::Error(unstarted(&error)));
                continue;
            }
        };
        let pending = run.pending.enter();
        match activate::activate_agent(&daemon, &run).await {
            Ok(Activated::Open) => {}
            Ok(Activated::Ending) => {
                drop(pending);
                run.gone().await;
                continue;
            }
            Err(error) => {
                answer(daemon.live.take_waiting(id).await, &Fate::Error(error));
                continue;
            }
        }
        // Rules 4 and 5.
        let batch = daemon.live.take_waiting(id).await;
        let mut requests: Vec<(enqueue::request::Item, Sent)> = Vec::new();
        for waiting in batch {
            events::record(&daemon, &waiting.sender, &run.sender).await;
            run.messages.lock().await.insert(waiting.key.clone(), Message::new(waiting.sender));
            match waiting.what {
                What::Chat(content) => {
                    let message = enqueue::request::Message { key: waiting.key.clone(), content };
                    match requests.last_mut() {
                        Some((enqueue::request::Item::Chat { messages }, sent)) => {
                            messages.push(message);
                            sent.keys.push(waiting.key);
                            sent.fates.push(waiting.fate);
                        }
                        _ => requests.push((
                            enqueue::request::Item::Chat { messages: vec![message] },
                            Sent { keys: vec![waiting.key], fates: vec![waiting.fate] },
                        )),
                    }
                }
                What::Decision { content, questions } => requests.push((
                    enqueue::request::Item::StructuredDecisions(Decision { key: waiting.key.clone(), content, questions }),
                    Sent { keys: vec![waiting.key], fates: vec![waiting.fate] },
                )),
            }
        }
        run.touch();
        // Rule 6: every request out at once, each under its index.
        let mut unanswered: HashMap<u64, Sent> = HashMap::new();
        let mut asking = FuturesUnordered::new();
        for (item, sent) in requests {
            let index = run.next_index().await;
            unanswered.insert(index, sent);
            let handle = &run.handle;
            asking.push(async move { (index, handle.enqueue(enqueue::request::Request { index, item }).await) });
        }
        let failed = loop {
            tokio::select! {
                answered = asking.next() => {
                    let Some((index, frame)) = answered else {
                        break None;
                    };
                    let fate = match frame {
                        Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
                        Ok(enqueue::response::Frame::Error(error)) => Fate::Error(error),
                        Err(error) => Fate::Error(Error(serde_json::json!({
                            "kind": "daemon",
                            "error": format!("the messages could not be enqueued: {error}"),
                        }))),
                    };
                    if let Some(sent) = unanswered.remove(&index) {
                        settle(&run, sent, &fate).await;
                    }
                }
                () = run.unfed.notified() => {
                    if let Err(error) = activate::activate_agent(&daemon, &run).await {
                        stop::fail_agent(&run).await;
                        break Some(Fate::Error(error));
                    }
                }
            }
        };
        drop(asking);
        // A feed that failed: every request not yet answered has that
        // error for its fate.
        if let Some(fate) = failed {
            for (_, sent) in unanswered {
                settle(&run, sent, &fate).await;
            }
        }
        drop(pending);
    }
}

/// One request answered: its entries marked or removed as rules 2 and
/// 3 of [`Message`] state — an undelivered entry leaves now, a
/// delivered one is the run's pump's to sweep — and its fate said to
/// every message it carried.
async fn settle(run: &super::AgentRun, sent: Sent, fate: &Fate) {
    {
        let mut kept = run.messages.lock().await;
        for key in &sent.keys {
            if matches!(fate, Fate::Delivered) {
                if let Some(message) = kept.get_mut(key) {
                    message.delivered = true;
                }
            } else {
                kept.remove(key);
            }
        }
    }
    for said in sent.fates {
        let _ = said.send(fate.clone());
    }
}

/// `fate`, said to every message of `waiting`: one never sent, since
/// no run could be started or activated for it. A handler that is gone
/// is a fate nobody hears.
fn answer(waiting: Vec<Waiting>, fate: &Fate) {
    for waiting in waiting {
        let _ = waiting.fate.send(fate.clone());
    }
}

/// A start that failed, as the error every message waiting on it is
/// answered with: what a provider's capabilities lacked, why no
/// provider can source the image, or the daemon's failure, in the
/// start's own words.
fn unstarted(error: &StartError) -> Error {
    match (error.unsupported(), error.no_source()) {
        (Some(what), _) => reply::unsupported(what),
        (None, Some(what)) => reply::no_source(what),
        (None, None) => reply::failure(error),
    }
}
