//! Keeping a tool's log as the daemon routes into the tool.
//!
//! The rules, as [`tool_log::Item`] states them:
//!
//! 1. An ask is kept by [`begin`] before the tool is asked, and its
//!    answer by [`end`] once the tool answers or fails to, naming the
//!    ask by the index `begin` handed back.
//! 2. `begin` keeps `active` first when no ask is in flight, and `end`
//!    keeps `inactive` after the answer that leaves none; the count of
//!    asks in flight is held under the log's own lock, so the two
//!    marks land exactly between the asks they bracket.
//! 3. [`note`] keeps a notification or an error, and moves nothing in
//!    flight.
//! 4. A log that cannot be written fails nothing it records: `begin`
//!    hands back `None`, the ask goes on unlogged and uncounted, and
//!    its answer is not kept; an answer or a note that cannot be
//!    written is let go.
//!
//! [`session()`] is rule 3's notifications: one reader of what the tool
//! sends on each caller's MCP session, started at that session's first
//! use, so each notification is kept once, naming whom it reached.

use std::sync::Arc;

use diverge_sdk::daemon::tool_log;
use futures_util::StreamExt as _;
use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::shared::mcp::session;
use rmcp::model::ErrorData;

use super::{ToolHandle, ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::logs;

/// Rules 1, 2 and 4: keep `ask`, after `active` if nothing was in
/// flight, and count it in flight; its index, or `None` when the log
/// could not be written.
pub async fn begin(daemon: &Daemon, key: ToolKey, ask: tool_log::Item) -> Option<u64> {
    let log = daemon.live.tool_log(key).await;
    let mut flying = log.lock.lock().await;
    if *flying == 0 {
        let wrapper = logs::append(&daemon.logs, key, tool_log::Item::Active).await.ok()?;
        log.latest.send_replace(wrapper.logs_index);
    }
    let wrapper = logs::append(&daemon.logs, key, ask).await.ok()?;
    log.latest.send_replace(wrapper.logs_index);
    *flying += 1;
    Some(wrapper.logs_index)
}

/// Rules 1, 2 and 4: keep `answer`, count its ask out of flight, and
/// keep `inactive` when none is left. An ask `begin` did not keep has
/// no answer kept.
pub async fn end(daemon: &Daemon, key: ToolKey, ask: Option<u64>, answer: impl FnOnce(u64) -> tool_log::Item) {
    let Some(ask) = ask else {
        return;
    };
    let log = daemon.live.tool_log(key).await;
    let mut flying = log.lock.lock().await;
    if let Ok(wrapper) = logs::append(&daemon.logs, key, answer(ask)).await {
        log.latest.send_replace(wrapper.logs_index);
    }
    *flying = flying.saturating_sub(1);
    if *flying == 0
        && let Ok(wrapper) = logs::append(&daemon.logs, key, tool_log::Item::Inactive).await
    {
        log.latest.send_replace(wrapper.logs_index);
    }
}

/// Rules 3 and 4: keep `item` — a notification or an error.
pub async fn note(daemon: &Daemon, key: ToolKey, item: tool_log::Item) {
    let log = daemon.live.tool_log(key).await;
    let _held = log.lock.lock().await;
    if let Ok(wrapper) = logs::append(&daemon.logs, key, item).await {
        log.latest.send_replace(wrapper.logs_index);
    }
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
/// stream ends — the reader among the run's tasks, so the run's end
/// ends it. A connected tool's sessions are the far daemon's, which
/// keeps what it sends; nothing is read here.
pub async fn session(daemon: &Arc<Daemon>, run: &Arc<ToolRun>, caller: &Caller) {
    if let ToolHandle::Connected(_) = run.handle {
        return;
    }
    if !run.sessions.lock().await.insert(session::key(caller)) {
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
    run.tasks.lock().await.push(task.abort_handle());
}
