//! A run activated: its secrets taken and fed before work reaches it
//! — an agent's feed is the go as well, sent and answered before its
//! batch of messages leaves; and gone inactive: what it made of them
//! written back.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::client::answered::Answered;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use tokio::sync::Mutex;

use super::secrets::{self, Secrets};
use super::{AgentRun, Gate, ToolHandle, ToolRun};
use crate::daemon::Daemon;

/// What an agent's activation came to, where it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activated {
    /// The gate is open: the batch may be sent.
    Open,
    /// The daemon is ending the run — its idle clock, or a feed that
    /// failed under a batch: nothing is to be sent to it, and the
    /// batch is the next run's.
    Ending,
}

/// The agent's run activated for the batch about to be sent to it —
/// its proxy's gate opened, which is the go: the proxy launches the
/// loop on the first batch that reaches an open gate, and hands a
/// loop in flight every batch after. Called by the agent's outbox
/// pump alone, with the run's `pending` held — see
/// [`message`](super::message) — under the run's gate lock, so an
/// activation, an inactive and the idle clock's last look are served
/// one after the other:
///
/// 1. A run the daemon is ending — the gate's `ending`, set under
///    this lock by [`idle`](super::idle)'s last look, or by the pump
///    for a run it could not feed again — is not activated:
///    [`Ending`](Activated::Ending), and nothing is fed. The clock
///    looked before the pump held the run, so the pump's hold came
///    too late to stop it; a hold it saw stops it instead, and this
///    is then never the answer.
/// 2. A gate open already is left so: nothing is done, and the batch
///    is the loop's, or the next one's.
/// 3. Else the run is fed: its values taken and fed as rule 3 of
///    [`activate`] says, where its program registered any secret; an
///    empty feed, where it registered nothing. The gate is open once
///    the provider's `Fed` is back, which is waited for: the batch
///    leaves only after it.
/// 4. A take or a feed that fails leaves the gate shut, releases
///    whatever was taken, and is the failure, kind `secrets`.
pub async fn activate_agent(daemon: &Daemon, run: &AgentRun) -> Result<Activated, Error> {
    let mut gate = run.gate.lock().await;
    if gate.ending {
        return Ok(Activated::Ending);
    }
    if !gate.open {
        open(daemon, &mut gate, run).await?;
    }
    Ok(Activated::Open)
}

/// The tool's run activated for an MCP exchange about to be put to
/// it, by the rules of `activate` below. The failure is the
/// exchange's, kind `secrets`.
pub async fn activate_tool(daemon: &Daemon, run: &ToolRun) -> Result<(), Error> {
    activate(daemon, &run.secrets, &run.sender, Feed::Tool(&run.handle)).await
}

/// The agent's container went inactive with `values`, what its
/// program made of its rotating secrets — its proxy's gate shut with
/// the word — under the run's gate lock:
///
/// 1. The gate is marked shut.
/// 2. What the program made of its secrets is written back, as rule
///    2 of [`inactive`] says, where it registered any; a write-back
///    the store refused is the error, for the log.
/// 3. Then, while the agent's outbox pump holds the run — `pending`:
///    a batch on its way to it, activated for and not yet answered —
///    the gate is opened again at once, as [`activate_agent`] does:
///    such a batch crossed this inactive, and is at the proxy behind
///    the shut gate, or about to be, so the feed is all it waits for.
///    A run the daemon is ending is fed nothing.
/// 4. A failure of either leaves the gate shut, and is told: the
///    run's pump keeps it in the log and wakes the outbox pump, which
///    activates the run again for the batch it waits on — rule 5 of
///    [`message`](super::message)'s pump.
pub async fn inactive_agent(daemon: &Daemon, run: &AgentRun, values: Values) -> Result<(), Error> {
    let mut gate = run.gate.lock().await;
    gate.open = false;
    if let Some(state) = gate.secrets.as_mut() {
        secrets::write_back(daemon, state, &run.sender, &values)
            .await
            .map_err(|error| refused(&format!("the values could not be written back: {error}")))?;
    }
    if gate.ending || !run.pending.is_busy() {
        return Ok(());
    }
    open(daemon, &mut gate, run).await
}

/// The agent's gate opened, as rules 3 and 4 of [`activate_agent`] say.
async fn open(daemon: &Daemon, gate: &mut Gate, run: &AgentRun) -> Result<(), Error> {
    let feed = Feed::Agent(&run.handle);
    match gate.secrets.as_mut() {
        Some(state) => take_and_feed(daemon, state, &run.sender, &feed).await?,
        None => feed
            .feed(Values::new())
            .await
            .map_err(|reason| refused(&format!("the container could not be fed: {reason}")))?,
    }
    gate.open = true;
    Ok(())
}

/// The tool's container went inactive with `values`, by the rules of
/// `inactive` below. The run is activated again at once while any
/// exchange is in flight into it, for the same reason.
pub async fn inactive_tool(daemon: &Daemon, run: &ToolRun, values: Values) -> Result<(), Error> {
    inactive(daemon, &run.secrets, &run.sender, values, || run.inflight.is_busy(), Feed::Tool(&run.handle)).await
}

/// The run's scope, as its values are fed down it.
enum Feed<'a> {
    /// An agent's run.
    Agent(&'a AgentHandle),
    /// A tool's run, or the connect scope on another daemon's tool,
    /// which is fed nothing: that daemon feeds its own container.
    Tool(&'a ToolHandle),
}

impl Feed<'_> {
    /// The values fed to the container and applied there, the
    /// provider's `Fed` waited for; why not, in a sentence that never
    /// carries a value.
    async fn feed(&self, values: Values) -> Result<(), String> {
        match self {
            Feed::Agent(handle) => handle.secrets(values).await.map_err(refusal),
            Feed::Tool(ToolHandle::Run(handle)) => handle.secrets(values).await.map_err(refusal),
            Feed::Tool(ToolHandle::Connected(_)) => Ok(()),
        }
    }
}

/// The rules of a tool's activation — an idle run about to receive an
/// MCP exchange — under the run's secrets lock, so two deliveries at
/// once activate it once:
///
/// 1. A run whose program registered nothing — no `secrets` state at
///    all — is fed nothing: it opened no registration channel, and
///    the proxy's gate is open from its begin.
/// 2. A run holding slots already is active: nothing is done, and the
///    work is delivered.
/// 3. Else the values are taken — [`secrets::acquire`], one slot held
///    per rotating name — and fed down the run's `secrets` channel,
///    the provider's `Fed` awaited; the proxy opens its gate on that,
///    and the work is delivered after. A feed happens at every
///    activation, declaration empty or not, since the gate opens only
///    on `Fed`. An agent's run is fed the same way: see
///    [`activate_agent`].
/// 4. A take or a feed that fails releases whatever was taken and is
///    the delivery's error, kind `secrets`, naming secrets and keys
///    and never a value.
async fn activate(daemon: &Daemon, secrets: &Mutex<Option<Secrets>>, holder: &Creator, feed: Feed<'_>) -> Result<(), Error> {
    let mut guard = secrets.lock().await;
    let Some(state) = guard.as_mut() else {
        return Ok(());
    };
    if state.held.is_some() {
        return Ok(());
    }
    take_and_feed(daemon, state, holder, &feed).await
}

/// The rules of a tool's inactive — the pump read `Inactive(values)`
/// off the run's stream — under the same lock:
///
/// 1. A run whose program registered nothing has nothing to write
///    back, and is done.
/// 2. Each rotating value in `values` is written into the slot the
///    run holds for its name and the hold dropped —
///    [`secrets::write_back`]; a fixed name in the map is ignored; a
///    rotating name absent from it is released unchanged, which the
///    proxy refuses before it gets here.
/// 3. Then, while work is still outstanding — `busy`, asked under the
///    lock: the exchanges in flight — the run is activated again at
///    once, as
///    [`activate`] does, so work that crossed the inactive in flight
///    is not stuck behind the proxy's gate.
/// 4. A write-back that the store refused, or a take or a feed that
///    failed, is the error, kind `secrets`, for the log: the run
///    holds nothing after a refused write-back, and the next delivery
///    activates it afresh.
async fn inactive(daemon: &Daemon, secrets: &Mutex<Option<Secrets>>, holder: &Creator, values: Values, busy: impl FnOnce() -> bool, feed: Feed<'_>) -> Result<(), Error> {
    let mut guard = secrets.lock().await;
    let Some(state) = guard.as_mut() else {
        return Ok(());
    };
    secrets::write_back(daemon, state, holder, &values)
        .await
        .map_err(|error| refused(&format!("the values could not be written back: {error}")))?;
    if !busy() {
        return Ok(());
    }
    take_and_feed(daemon, state, holder, &feed).await
}

/// The values taken and fed, as rules 3 and 4 of [`activate`] say.
async fn take_and_feed(daemon: &Daemon, state: &mut Secrets, holder: &Creator, feed: &Feed<'_>) -> Result<(), Error> {
    let values = secrets::acquire(daemon, state, holder).await.map_err(|error| refused(&error.to_string()))?;
    if let Err(reason) = feed.feed(values).await {
        let _ = secrets::release(daemon, state, holder).await;
        return Err(refused(&format!("the values could not be fed to the container: {reason}")));
    }
    Ok(())
}

/// Why a feed failed, in a sentence: the program's refusal as the
/// proxy worded it — never a value — or this end's failure as it says
/// itself.
fn refusal<A: Answered<Refusal = Error>>(error: UnaryError<A>) -> String {
    match error {
        UnaryError::Refused(error) => format!("the program refused the values: {}", error.0),
        error => error.to_string(),
    }
}

/// The delivery's error: kind `secrets`.
fn refused(what: &str) -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": what,
    }))
}
