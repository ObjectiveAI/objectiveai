//! Keeping a tool's log as the daemon routes into the tool.
//!
//! The rules, as [`tool_log::Item`] states them, kept through the
//! run's [`lifecycle`]:
//!
//! 1. An ask is kept by [`begin`] before the tool is asked, and only on
//!    a deployed run's log — [`lifecycle::begin`] — and its answer by
//!    [`end`] once the tool answers or fails to, naming the ask by the
//!    index `begin` handed back. An answer that is the run going down
//!    keeps nothing: the run's `undeployed` answers the ask as
//!    interrupted.
//! 2. `begin` keeps `active` first when no ask is open, and `end` keeps
//!    `inactive` after the answer that leaves none; the tool is marked
//!    working, its `last_active` kept and its list told, on exactly
//!    those two, and on an `undeployed` that closed it working.
//! 3. [`note`] keeps a notification or an error, and moves nothing.
//! 4. A log that cannot be written fails nothing it records: `begin`
//!    hands back `None`, the ask goes on unlogged, and its answer is
//!    not kept; an answer or a note that cannot be written is let go.
//!
//! [`session()`] is rule 3's notifications: one reader of what the tool
//! sends on each caller's MCP session, started at that session's first
//! use, so each notification is kept once, naming whom it reached. A
//! session's reader lives no longer than the session: see
//! [`end_session`] and [`forget_agent`]; and a session that ends is
//! closed in its tool as well: see [`close_session`].

use std::sync::Arc;

use diverge_sdk::daemon::tool_log;
use futures_util::StreamExt as _;
use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::mcp::attestation::{self, Caller};
use diverge_sdk::shared::mcp::session;
use rmcp::model::ErrorData;

use super::lifecycle::{self, Answer, Ask};
use super::{ToolHandle, ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::logs;

/// Rules 1, 2 and 4: keep `ask` on `run`'s log, after `active` if
/// nothing was open; its index, or `None` when it was not kept.
pub async fn begin(daemon: &Daemon, run: &ToolRun, ask: tool_log::Item, kind: Ask) -> Option<u64> {
    let (index, began) = lifecycle::begin(daemon.logs.clone(), run.id, Arc::clone(&run.log), run.epoch, ask, kind).await?;
    if began {
        working(daemon, run.id, true).await;
    }
    Some(index)
}

/// Rules 1, 2 and 4: keep the answer to the ask at `ask`, and
/// `inactive` when none is left open. An ask `begin` did not keep has
/// no answer kept.
pub async fn end(daemon: &Daemon, run: &ToolRun, ask: Option<u64>, answer: impl FnOnce(u64) -> Answer<tool_log::Item>) {
    let Some(ask) = ask else {
        return;
    };
    if lifecycle::end(daemon.logs.clone(), run.id, Arc::clone(&run.log), run.epoch, ask, answer(ask)).await {
        working(daemon, run.id, false).await;
    }
}

/// The tool began or ceased working: marked so, now kept as its
/// `last_active`, and its list told. A time that cannot be kept fails
/// nothing.
pub async fn working(daemon: &Daemon, key: ToolKey, busy: bool) {
    daemon.live.set_busy(key, busy).await;
    if let Ok(mut conn) = daemon.store.acquire().await {
        let now = chrono::Utc::now();
        let _ = match key {
            ToolKey::Record(id) => crate::store::tools::set_active(&mut conn, id, now).await,
            ToolKey::Connected(id) => crate::store::connected_standalone_tools::set_active(&mut conn, id, now).await,
            ToolKey::Dependency(id) => crate::store::dependency_tools::set_active(&mut conn, id, now).await,
        };
    }
    daemon.live.changed(match key {
        ToolKey::Record(_) => crate::daemon::Kind::StandaloneTools,
        ToolKey::Connected(_) => crate::daemon::Kind::ConnectedStandaloneTools,
        ToolKey::Dependency(_) => crate::daemon::Kind::DependencyTools,
    });
}

/// Rules 3 and 4: keep `item` — a notification, an error, a command.
pub async fn note(daemon: &Daemon, key: ToolKey, item: tool_log::Item) {
    let log = daemon.live.tool_log(&daemon.store, key).await;
    let _ = lifecycle::note(daemon.logs.clone(), key, log, item).await;
}

/// The tool is deleted: its live log forgotten, which ends every
/// watch on it, and its log removed. A log that cannot be removed is
/// left where it lies; the tool is gone either way.
pub async fn forget(daemon: &Daemon, key: ToolKey) {
    daemon.live.end_tool_log(key).await;
    let _ = logs::remove(&daemon.logs, key).await;
}

/// An MCP error, as the log keeps it: its own JSON.
pub fn error_data(error: &ErrorData) -> diverge_sdk::shared::error::Error {
    diverge_sdk::shared::error::Error(serde_json::to_value(error).unwrap_or(serde_json::Value::Null))
}

/// An error in the daemon's words, as the log keeps it.
pub fn words(error: &str) -> diverge_sdk::shared::error::Error {
    diverge_sdk::shared::error::Error(serde_json::Value::String(error.to_string()))
}

/// Rule 3: the first time `caller`'s session is used on the run, keep
/// every notification the tool sends on it, naming `caller`, until its
/// stream ends — the reader kept in the run's `sessions` under the
/// session's key, so the session's end ([`end_session`]), its agent's
/// deletion ([`forget_agent`]) and the run's end each end it. A
/// connected tool's sessions are the far daemon's, which keeps what it
/// sends; nothing is read here.
pub async fn session(daemon: &Arc<Daemon>, run: &Arc<ToolRun>, caller: &Caller) {
    if let ToolHandle::Connected(_) = run.handle {
        return;
    }
    let mut sessions = run.sessions.lock().await;
    let key = session::key(caller);
    if sessions.contains_key(&key) {
        return;
    }
    let daemon = Arc::clone(daemon);
    let tapped = Arc::clone(run);
    let to = caller.clone();
    let task = tokio::spawn(async move {
        let Ok(mut stream) = tapped.handle.notifications(&to).await else {
            return;
        };
        while let Some(Ok(notification)) = stream.next().await {
            let notification = serde_json::to_value(&notification).unwrap_or(serde_json::Value::Null);
            let item = tool_log::Item::Notification(tool_log::Notification { to: to.clone(), notification });
            note(&daemon, tapped.id, item).await;
        }
    });
    sessions.insert(key, task.abort_handle());
}

/// `caller`'s session on the run is over — the one connect scope it
/// was minted for has ended: its key forgotten and its reader ended,
/// and whether the key was there handed back. The one place a session
/// leaves a run's `sessions`, beside the run's end and
/// [`forget_agent`].
pub async fn end_session(run: &ToolRun, caller: &Caller) -> bool {
    let reader = run.sessions.lock().await.remove(&session::key(caller));
    match reader {
        Some(reader) => {
            reader.abort();
            true
        }
        None => false,
    }
}

/// `caller`'s session closed in the tool, on a task of its own. The
/// rules:
///
/// 1. The close is sent through the run's handle
///    ([`ToolHandle::close_session`]): to a created tool's container
///    proxy, and to nothing for a connected tool, whose session the
///    far daemon closes.
/// 2. Nothing waits for it: the task ends with the answer, or with the
///    run's scope, which ends the channel under it, so a proxy that
///    never answers holds no teardown and no task past the run.
/// 3. Its failure is let go: a session the close did not reach ends
///    with the container, and nothing of the daemon's depends on it.
pub fn close_session(run: &Arc<ToolRun>, caller: &Caller) {
    if let ToolHandle::Connected(_) = run.handle {
        return;
    }
    let run = Arc::clone(run);
    let caller = caller.clone();
    tokio::spawn(async move {
        let _ = run.handle.close_session(&caller).await;
    });
}

/// The agent is deleted: its session on every tool running now
/// forgotten, each reader ended, and each session the agent used
/// closed in its tool ([`close_session`]). The session's key names the
/// agent's key alone, so the image it is built with here is none.
pub async fn forget_agent(daemon: &Daemon, agent: key::Agent) {
    let caller = Caller::Agent(attestation::Agent { image: Image { digest: String::new(), references: Vec::new() }, key: agent });
    let runs = daemon.live.tool_runs().await;
    for run in runs {
        if end_session(&run, &caller).await {
            close_session(&run, &caller);
        }
    }
}

/// The run is over: every session's reader ended, and the sessions
/// forgotten.
pub async fn end_sessions(run: &ToolRun) {
    for (_, reader) in run.sessions.lock().await.drain() {
        reader.abort();
    }
}
