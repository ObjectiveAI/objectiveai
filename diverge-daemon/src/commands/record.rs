//! Keeping one request in its requester's book.

use std::sync::Arc;

use diverge_sdk::daemon::command_log::{self, Command};
use diverge_sdk::daemon::endpoints::ClientRequest;
use diverge_sdk::daemon::tool_log;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::wire::server::verdict::Verdict;

use super::{Commander, body};
use crate::containers::tool_log as tool_logs;
use crate::daemon::Daemon;
use crate::judge::Who;
use crate::logs;

/// Have the request kept when the daemon decides it.
///
/// The rules:
///
/// 1. A request the daemon could not read is not kept.
/// 2. The scope's [`Verdict`] decides `allowed`: the handler's word that
///    it let the request through, or a scope ended with no answer, is
///    `true`; a first answer is `false` exactly when it is the request's
///    own `Forbidden`, byte for byte.
/// 3. The command goes into the requester's book ([`Commander`]), kept
///    on a task of its own so the scope never waits on the log.
pub async fn record(scope: &ScopeHandle, request: &ClientRequest<'_>, payload: &[u8], who: &Who, daemon: &Arc<Daemon>) {
    // Rule 1.
    if let ClientRequest::Invalid(_) = request {
        return;
    }
    let Some(commander) = Commander::of(who) else {
        return;
    };
    let path = request.path();
    let forbidden = request.forbidden();
    let request = body(path, payload);
    let daemon = Arc::clone(daemon);
    scope
        .on_verdict(Box::new(move |verdict| {
            // Rule 2.
            let allowed = match verdict {
                Verdict::Allowed | Verdict::Nothing => true,
                Verdict::Response(bytes) => forbidden.as_deref() != Some(bytes),
            };
            let command = Command { path: path.to_string(), request, allowed };
            // Rule 3.
            tokio::spawn(async move { keep(&daemon, commander, command).await });
        }))
        .await;
}

/// The command into its book, under the book's live lock. A log that
/// cannot be written fails nothing it records.
async fn keep(daemon: &Daemon, commander: Commander, command: Command) {
    match commander {
        Commander::Account(id) => {
            let log = daemon.live.command_log(commander).await;
            let _held = log.lock.lock().await;
            if let Ok(wrapper) = logs::append(&daemon.logs, id, command_log::Item::Command(command)).await {
                log.latest.send_replace(wrapper.logs_index);
            }
        }
        Commander::Admin(key) => {
            let log = daemon.live.command_log(commander).await;
            let _held = log.lock.lock().await;
            if let Ok(wrapper) = logs::append(&daemon.logs, key, command_log::Item::Command(command)).await {
                log.latest.send_replace(wrapper.logs_index);
            }
        }
        Commander::Tool(key) => tool_logs::note(daemon, key, tool_log::Item::Command(command)).await,
    }
}
