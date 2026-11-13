//! A watched message: what the agent's log takes of it, sent as it
//! lands.

use diverge_sdk::daemon::agent_log::Item;
use diverge_sdk::daemon::endpoints::agents::message::client::channel_request;
use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::watch;

use crate::containers::message::{Admitted, Fate};
use crate::daemon::Daemon;
use crate::logs;
use crate::serve::logs::cancel;
use crate::serve::reply;
use crate::store::AgentId;

/// Answer a watched message, admitted as `admitted`, on `scope`: the
/// handler finishes the scope after. `latest` is the agent's log's
/// latest index, subscribed before the message was admitted, so that
/// the agent's deletion — the sender gone — ends the watch. The rules,
/// which are those of the
/// [response](diverge_sdk::daemon::endpoints::agents::message::server::response::Frame):
///
/// 1. **The fate first.** An error is the one answer; a fate never
///    said — the agent's pump died — is the daemon's error.
///    `Delivered` is not sent.
/// 2. **Then the echo.** The message's echo is waited for, as rule 4
///    of [`Message`](crate::containers::Message) states it: said, it
///    is the `logs_index` to send from; never said — the run's entry
///    for the message gone first — the one answer is `Dropped`.
/// 3. **Then the log, from the echo on**, each item as the log holds
///    it, read as far as the log goes and then again at each append,
///    up to and including the last:
///    - an `inactive`: the agent ceased working, the end of the run
///      the echo landed in;
///    - for a structured decision — the echo is one — the
///      `structured_decisions_response` whose `id` is the echo's;
///    - an `undeployed`, where no `inactive` came before it: the
///      run's end, kept as the last for a log that could not keep the
///      `inactive`.
/// 4. **A cancel ends it** wherever it waits — on the fate, on the
///    echo, on the next append — and so does the client's connection
///    going, and the agent's deletion. The message is not taken back.
/// 5. **A log that cannot be read** is the daemon's error, after the
///    items sent.
pub async fn watch(scope: &ScopeHandle, daemon: &Daemon, id: AgentId, admitted: Admitted, mut latest: watch::Receiver<u64>) {
    // Rule 1.
    let fate = tokio::select! {
        fate = admitted.fate => fate,
        _ = cancel(scope, is_cancel) => return,
    };
    match fate {
        Ok(Fate::Delivered) => {}
        Ok(Fate::Error(error)) => return reply::reply(scope, &Frame::Error(error)).await,
        Err(_) => {
            return reply::reply(scope, &Frame::Error(reply::failure(&"the message's delivery ended before its fate was known"))).await;
        }
    }
    // Rule 2.
    let echo = tokio::select! {
        echo = admitted.echo => echo,
        _ = cancel(scope, is_cancel) => return,
    };
    let Ok(mut next) = echo else {
        return reply::reply(scope, &Frame::Dropped).await;
    };
    // Rule 3. The decision's id, once its echo is read.
    let mut decision: Option<u64> = None;
    loop {
        let read = match logs::count(&daemon.logs, id).await {
            Ok(count) if next <= count => logs::read(&daemon.logs, id, next, count).await.map(|items| (items, count)),
            Ok(count) => Ok((Vec::new(), count)),
            Err(error) => Err(error),
        };
        let (items, count) = match read {
            Ok(read) => read,
            Err(error) => return reply::reply(scope, &Frame::Error(reply::failure(&error))).await,
        };
        for item in items {
            let last = match &item.item {
                Item::UserStructuredDecisions(echoed) if decision.is_none() => {
                    decision = Some(echoed.id);
                    false
                }
                Item::StructuredDecisionsResponse(response) => decision == Some(response.id),
                Item::Inactive(_) | Item::Undeployed(_) => true,
                _ => false,
            };
            reply::reply(scope, &Frame::Item(item)).await;
            if last {
                return;
            }
        }
        next = next.max(count + 1);
        // Rule 4.
        tokio::select! {
            changed = latest.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            _ = cancel(scope, is_cancel) => return,
        }
    }
}

/// Whether a channel request's payload is the cancel.
fn is_cancel(payload: &[u8]) -> bool {
    matches!(channel_request::Frame::decode(payload), Ok(channel_request::Frame::Cancel))
}
