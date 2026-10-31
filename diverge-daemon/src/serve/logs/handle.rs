//! The `logs` endpoint: whose log, judged, then sent.

use diverge_sdk::daemon::agent_log;
use diverge_sdk::daemon::command_log;
use diverge_sdk::daemon::endpoints::logs::client::channel_request;
use diverge_sdk::daemon::endpoints::logs::client::request::{self, Subject};
use diverge_sdk::daemon::endpoints::logs::server::response::{Entry, Frame};
use diverge_sdk::daemon::grant;
use diverge_sdk::daemon::tool_log;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, Span, send};
use crate::commands::{AdminKey, Commander};
use crate::containers::ToolKey;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{agents, dependency_tools, reply, standalone_tools};
use crate::store;

/// Send what the filter yields, watch if asked, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// Judge the subject by its own family's `logs` grant and send its log.
///
/// The rules:
///
/// 1. No standing, no `logs` grant over the subject's family at all, or
///    a subject the grants do not reach, is `Forbidden`.
/// 2. A subject that is not there is the scope's error.
/// 3. Else the log is sent, and watched if asked, as
///    [`logs`](crate::serve::logs) states, each item wrapped in the
///    subject's kind; the scope is marked let through first, since a
///    watch's first answer may be long in coming.
/// 4. An administrator's command log is read by administrators alone.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), Failure> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let span = Span {
        from: frame.logs_index_from,
        to: frame.logs_index_to,
        created_to: frame.created_to,
        count: frame.count,
        watch: frame.watch,
    };
    let (r#type, from, to) = (frame.r#type, frame.created_from, frame.created_to);
    match &frame.of {
        Subject::Agent(reference) => {
            if !judge::agents::holds(&standing, grant::agents::Over::Logs) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some(agent) = store::agents::by_reference(&mut conn, reference, false).await? else {
                reply::reply(scope, &Frame::Error(reply::failure(&"the agent named is none the daemon has"))).await;
                return Ok(());
            };
            drop(conn);
            if !judge::agents::over(&standing, grant::agents::Over::Logs, &agent, agents::state(daemon, agent.id).await) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            scope.allowed().await;
            // Subscribe before the first read; the handle itself is let
            // go, so the agent's deletion — the sender gone — ends the
            // watch.
            let latest = daemon.live.log(agent.id).await.latest.subscribe();
            let keep = |item: &agent_log::ItemWrapper| crate::logs::matches(r#type, from, to, agent_log::ItemType::of(&item.item).name(), item.created);
            send(scope, daemon, agent.id, latest, &span, keep, |item| Frame::Item(Entry::Agent(item)), is_cancel).await?;
        }
        Subject::StandaloneTool(reference) => {
            if !judge::standalone_tools::holds(&standing, grant::standalone_tools::Over::Logs) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some(tool) = standalone_tools::resolve(&mut conn, reference, false).await? else {
                reply::reply(scope, &Frame::Error(reply::failure(&"the tool named is none the daemon has"))).await;
                return Ok(());
            };
            if !standalone_tools::reaches(&mut conn, daemon, &standing, grant::standalone_tools::Over::Logs, &tool).await? {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            drop(conn);
            scope.allowed().await;
            let key = ToolKey::Record(tool.id);
            let latest = daemon.live.tool_log(key).await.latest.subscribe();
            let keep = |item: &tool_log::ItemWrapper| crate::logs::matches(r#type, from, to, tool_log::ItemType::of(&item.item).name(), item.created);
            send(scope, daemon, key, latest, &span, keep, |item| Frame::Item(Entry::StandaloneTool(item)), is_cancel).await?;
        }
        Subject::DependencyTool(reference) => {
            if !judge::dependency_tools::holds(&standing, grant::dependency_tools::Over::Logs) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some((tool, _)) = dependency_tools::resolve(&mut conn, daemon, reference, false).await? else {
                reply::reply(scope, &Frame::Error(reply::failure(&"the dependency tool named is none the daemon has"))).await;
                return Ok(());
            };
            let Some(loaded) = dependency_tools::load(&mut conn, daemon, &tool).await? else {
                reply::reply(scope, &Frame::Error(reply::failure(&"the dependency tool named is none the daemon has"))).await;
                return Ok(());
            };
            if !judge::dependency_tools::over(&standing, grant::dependency_tools::Over::Logs, &loaded.facts(&tool)) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            drop(conn);
            scope.allowed().await;
            let key = ToolKey::Dependency(tool.id);
            let latest = daemon.live.tool_log(key).await.latest.subscribe();
            let keep = |item: &tool_log::ItemWrapper| crate::logs::matches(r#type, from, to, tool_log::ItemType::of(&item.item).name(), item.created);
            send(scope, daemon, key, latest, &span, keep, |item| Frame::Item(Entry::DependencyTool(item)), is_cancel).await?;
        }
        Subject::Account(identity) => {
            if !judge::accounts::holds(&standing, grant::accounts::Over::Logs) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            let Some(account) = store::accounts::by_identity(&mut conn, identity, false).await? else {
                reply::reply(scope, &Frame::Error(reply::failure(&"the account named is none the daemon has"))).await;
                return Ok(());
            };
            drop(conn);
            let connected = daemon.live.is_connected(account.id).await;
            if !judge::accounts::over(&standing, grant::accounts::Over::Logs, &account, connected) {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            scope.allowed().await;
            let latest = daemon.live.command_log(Commander::Account(account.id)).await.latest.subscribe();
            let keep = |item: &command_log::ItemWrapper| crate::logs::matches(r#type, from, to, command_log::ItemType::of(&item.item).name(), item.created);
            send(scope, daemon, account.id, latest, &span, keep, |item| Frame::Item(Entry::Account(item)), is_cancel).await?;
        }
        Subject::Admin(identity) => {
            // Rule 4: an administrator's log is read by administrators
            // alone; an identity the daemon has kept nothing for reads
            // empty.
            if !standing.admin() {
                reply::reply(scope, &Frame::Forbidden).await;
                return Ok(());
            }
            drop(conn);
            let key = match store::hash::identity_key(identity) {
                Ok(key) => AdminKey(key),
                Err(error) => {
                    reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
                    return Ok(());
                }
            };
            scope.allowed().await;
            let latest = daemon.live.command_log(Commander::Admin(key)).await.latest.subscribe();
            let keep = |item: &command_log::ItemWrapper| crate::logs::matches(r#type, from, to, command_log::ItemType::of(&item.item).name(), item.created);
            send(scope, daemon, key, latest, &span, keep, |item| Frame::Item(Entry::Admin(item)), is_cancel).await?;
        }
    }
    Ok(())
}

/// Whether a channel request's payload is the cancel.
fn is_cancel(payload: &[u8]) -> bool {
    matches!(channel_request::Frame::decode(payload), Ok(channel_request::Frame::Cancel))
}
