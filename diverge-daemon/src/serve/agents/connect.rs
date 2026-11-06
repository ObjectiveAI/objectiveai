//! MCP as an agent: the agent's exchanges, answered on the scope's
//! channels as its own container's are.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::connect::client::channel_request;
use diverge_sdk::daemon::endpoints::agents::connect::client::request;
use diverge_sdk::daemon::endpoints::agents::connect::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::shared::mcp;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;
use rmcp::model::ErrorData;
use tokio::task::JoinSet;

use super::state;
use crate::containers::{self, AgentRun};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, AgentId, agents};

/// Answer the connect, serve its channels until the client disconnects
/// or its connection ends, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let scope = Arc::new(scope);
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged first.** `Forbidden` with no `connect` grant at all;
///    `NotFound` for no such agent; `Forbidden` for an agent the
///    grants do not reach. No account is asked for: what is stamped
///    on every exchange is the agent, not the client.
/// 2. **Then connected.** Exactly one `Connected`, and nothing started:
///    the container is the exchanges' to start.
/// 3. **Every channel is one exchange,** answered on a task of its own
///    (see [`answer`]); a disconnect, or the client's connection
///    ending, ends the scope, and every exchange still in flight with
///    it; a channel request that does not decode is finished with
///    nothing.
async fn serve(scope: &Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    // Rule 1.
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Connect) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Connect, &agent, state(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    scope.allowed().await;
    // Rule 2.
    reply::reply(scope, &Frame::Connected).await;
    // Rule 3.
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Disconnect) => break,
                    Ok(ask) => {
                        tasks.spawn(answer(Arc::clone(daemon), Arc::clone(scope), agent.id, channel, ask));
                    }
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            // Every answered exchange reaped as it ends, so the set
            // holds only those in flight.
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    // The scope mints no session of its own: its exchanges are the
    // agent's, on the agent's own session in each tool, which goes
    // with the agent's deletion (`tool_log::forget_agent`), not here.
    tasks.abort_all();
    Ok(())
}

/// The agent's run, started as a message starts it when it is not up
/// — with every tool it depends on, waited for — and the agent read
/// again for it, so one deleted since the connect is the failure.
async fn deployed(daemon: &Arc<Daemon>, id: AgentId) -> Result<Arc<AgentRun>, ErrorData> {
    if let Some(run) = daemon.live.agent_run(id).await {
        return Ok(run);
    }
    let failed = |error: &dyn std::fmt::Display| ErrorData::internal_error(error.to_string(), None);
    let mut conn = daemon.store.acquire().await.map_err(|error| failed(&error))?;
    let agent = agents::by_id(&mut conn, id, false).await.map_err(|error| failed(&error))?;
    drop(conn);
    let Some(agent) = agent else {
        return Err(failed(&"the agent is gone"));
    };
    containers::agent(daemon, &agent).await.map_err(|error| failed(&error))
}

/// One channel, answered as the agent's own container's exchange is.
/// The rules:
///
/// 1. **Deployed first.** The agent's run, started if it is not up and
///    waited for (see [`deployed`]); one that will not start is the
///    exchange's error, and nothing else is done.
/// 2. **In flight for the agent.** A list, a call or a read is in
///    flight for the agent from its start until it is answered, and
///    touches the run: the agent is active meanwhile, and its idle
///    clock counts from the answer. The notifications are not in
///    flight, as the container's own are not.
/// 3. **The agent's own answer.** The exchange is put to the merge
///    with the agent's own context — its tools by the names it sees
///    them by, its stamp, its sessions, its log entries in each tool's
///    log — exactly as its container's would be.
/// 4. **Sent.** One channel response and the finish; the notifications
///    one per notification until their stream ends, then the finish.
async fn answer(daemon: Arc<Daemon>, scope: Arc<ScopeHandle>, id: AgentId, channel: u32, ask: channel_request::Frame) {
    // Rule 1.
    let run = match deployed(&daemon, id).await {
        Ok(run) => run,
        Err(error) => {
            refuse(&scope, channel, &ask, error).await;
            return;
        }
    };
    let context = run.context(&daemon);
    match ask {
        channel_request::Frame::Disconnect => {}
        // Rules 2 to 4.
        channel_request::Frame::McpListTools(request) => {
            let _flying = run.inflight.enter();
            run.touch();
            let frame = match containers::mcp::list_tools(&context, request.0).await {
                Ok(result) => mcp::list_tools::response::Frame::Result(result),
                Err(error) => mcp::list_tools::response::Frame::Error(error),
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpListResources(request) => {
            let _flying = run.inflight.enter();
            run.touch();
            let frame = match containers::mcp::list_resources(&context, request.0).await {
                Ok(result) => mcp::list_resources::response::Frame::Result(result),
                Err(error) => mcp::list_resources::response::Frame::Error(error),
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpCallTool(request) => {
            let _flying = run.inflight.enter();
            run.touch();
            let frame = match containers::mcp::call_tool(&context, request.0).await {
                Ok(result) => mcp::call_tool::response::Frame::Result(result),
                Err(error) => mcp::call_tool::response::Frame::Error(error),
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpReadResource(request) => {
            let _flying = run.inflight.enter();
            run.touch();
            let frame = match containers::mcp::read_resource(&context, request.0).await {
                Ok(result) => mcp::read_resource::response::Frame::Result(result),
                Err(error) => mcp::read_resource::response::Frame::Error(error),
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpNotifications(_) => {
            let mut stream = containers::mcp::notifications(&context).await;
            while let Some(item) = stream.next().await {
                let frame = match item {
                    Ok(notification) => mcp::notifications::response::Frame::Notification(notification),
                    Err(error) => mcp::notifications::response::Frame::Error(error),
                };
                let last = matches!(frame, mcp::notifications::response::Frame::Error(_));
                send(&scope, channel, &frame).await;
                if last {
                    break;
                }
            }
            scope.send_channel_response_finish(channel).await;
        }
    }
}

/// An exchange whose agent would not deploy: its error, as the
/// exchange's own frame, then the finish.
async fn refuse(scope: &ScopeHandle, channel: u32, ask: &channel_request::Frame, error: ErrorData) {
    match ask {
        channel_request::Frame::Disconnect => {}
        channel_request::Frame::McpListTools(_) => respond(scope, channel, &mcp::list_tools::response::Frame::Error(error)).await,
        channel_request::Frame::McpListResources(_) => respond(scope, channel, &mcp::list_resources::response::Frame::Error(error)).await,
        channel_request::Frame::McpCallTool(_) => respond(scope, channel, &mcp::call_tool::response::Frame::Error(error)).await,
        channel_request::Frame::McpReadResource(_) => respond(scope, channel, &mcp::read_resource::response::Frame::Error(error)).await,
        channel_request::Frame::McpNotifications(_) => respond(scope, channel, &mcp::notifications::response::Frame::Error(error)).await,
    }
}

/// One channel response, then the finish: a unary exchange's answer.
async fn respond<F: Encode>(scope: &ScopeHandle, channel: u32, frame: &F) {
    send(scope, channel, frame).await;
    scope.send_channel_response_finish(channel).await;
}

/// One channel response. A frame that will not encode is the one
/// failure with nowhere to go, and is not sent; the finish that every
/// answer ends with is what the client then reads.
async fn send<F: Encode>(scope: &ScopeHandle, channel: u32, frame: &F) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_channel_response(channel, &bytes).await;
    }
}
