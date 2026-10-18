//! A run activated: its secrets taken and fed before work reaches it;
//! and gone inactive: what it made of them written back.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::ExecuteHandle as AgentHandle;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::client::answered::Answered;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use tokio::sync::Mutex;

use super::secrets::{self, Secrets};
use super::{AgentRun, ToolHandle, ToolRun};
use crate::daemon::Daemon;

/// The agent's run activated for a message about to be enqueued, by
/// the rules of `activate` below. The failure is the message's, kind
/// `secrets`.
pub async fn activate_agent(daemon: &Daemon, run: &AgentRun) -> Result<(), Error> {
    activate(daemon, &run.secrets, &run.sender, Feed::Agent(&run.handle)).await
}

/// The tool's run activated for an MCP exchange about to be put to
/// it, by the rules of `activate` below. The failure is the
/// exchange's, kind `secrets`.
pub async fn activate_tool(daemon: &Daemon, run: &ToolRun) -> Result<(), Error> {
    activate(daemon, &run.secrets, &run.sender, Feed::Tool(&run.handle)).await
}

/// The agent's container went inactive with `values`, what its
/// program made of its rotating secrets, by the rules of `inactive`
/// below. The run is
/// activated again at once while any message is enqueued and not yet
/// fated, so one that crossed the inactive in flight is not held
/// behind the proxy's gate.
pub async fn inactive_agent(daemon: &Daemon, run: &AgentRun, values: Values) -> Result<(), Error> {
    inactive(daemon, &run.secrets, &run.sender, values, || run.pending.is_busy(), Feed::Agent(&run.handle)).await
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

/// The rules of an activation — an idle run about to receive work: an
/// agent a message, a tool an MCP exchange — under the run's secrets
/// lock, so two deliveries at once activate it once:
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
///    on `Fed`.
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

/// The rules of an inactive — the pump read `Inactive(values)` off
/// the run's stream — under the same lock:
///
/// 1. A run whose program registered nothing has nothing to write
///    back, and is done.
/// 2. Each rotating value in `values` is written into the slot the
///    run holds for its name and the hold dropped —
///    [`secrets::write_back`]; a fixed name in the map is ignored; a
///    rotating name absent from it is released unchanged, which the
///    proxy refuses before it gets here.
/// 3. Then, while work is still outstanding — `busy`, asked under the
///    lock: an agent's messages enqueued and not yet fated, a tool's
///    exchanges in flight — the run is activated again at once, as
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
