//! Serving a tool to another daemon: its container up and held, and
//! its MCP exchanges answered on the scope's channels.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::events::Action;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::channel_request;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::connect::server::response::Frame;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::shared::mcp::{self, attest_request};
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;
use rmcp::model::{ErrorData, GetMeta as _};
use tokio::task::JoinSet;

use super::{reaches, resolve};
use crate::containers::{self, ToolRun};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the connect, serve its channels until the client disconnects
/// or the tool's run ends, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let scope = Arc::new(scope);
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The one sentence a connected tool's connect answers with: it is
/// another daemon's, and that daemon serves it.
const CONNECTED: &str = "a connected tool is another daemon's, and is served there";

/// `Forbidden` with no `connect` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; the error for a connected
/// tool, which is another daemon's; the container started, or
/// touched if it runs, as a user
/// of the scope's own, which failing is the `Error`; else exactly one
/// `Connected`, telling the tool's image and template for the other
/// daemon to attest with, and then the scope held until the run ends
/// or the client disconnects: every channel the client opens is one
/// of the five MCP exchanges, put to the tool stamped with the caller
/// as this daemon authorized the connection — the identity its
/// handshake established, and nothing the client said of itself, in
/// place of
/// anything the client stamped — the tool's run activated for it
/// first, its secrets taken and fed — and answered by the tool's
/// server on a task of its own, each answer attested as this daemon's
/// own tool,
/// in place of anything the tool's server put; or the disconnect; a
/// channel request that does not decode is finished with nothing. On
/// either end the tasks are ended and the scope lets the tool go.
async fn serve(scope: &Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    // A dependency tool is no account, and a connect is attested as
    // the account the client is: a standing with no account is
    // refused.
    let (Some(identity), true) = (standing.identity.clone(), judge::standalone_tools::holds(&standing, Over::Connect)) else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let caller = Arc::new(Caller::Client { identity });
    let sender = Arc::new(standing.creator.clone());
    let Some(tool) = resolve(&mut conn, &frame.tool, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Connect, &tool).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(template) = tool.template() else {
        reply::reply(scope, &Frame::Error(reply::failure(&CONNECTED))).await;
        return Ok(());
    };
    let template = template.to_string();
    let connection = daemon.live.mint_connection().await;
    let run = match containers::use_tool(daemon, &tool).await {
        Ok(run) => run,
        Err(error) => {
            let error = match (error.unsupported(), error.no_source()) {
                (Some(what), _) => reply::unsupported(what),
                (None, Some(what)) => reply::no_source(what),
                (None, None) => reply::failure(&error),
            };
            reply::reply(scope, &Frame::Error(error)).await;
            return Ok(());
        }
    };
    containers::connect(&run, connection).await;
    reply::reply(
        scope,
        &Frame::Connected {
            image: run.attestation.image().clone(),
            template,
        },
    )
    .await;
    let mut tasks = JoinSet::new();
    let mut ended = run.ended.subscribe();
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
                        tasks.spawn(answer(Arc::clone(daemon), Arc::clone(scope), Arc::clone(&run), Arc::clone(&caller), Arc::clone(&sender), channel, ask));
                    }
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            () = over(&mut ended) => break,
        }
    }
    tasks.abort_all();
    containers::disconnect(daemon, &run, connection).await;
    Ok(())
}

/// Resolves when the run has ended, or its word is gone.
async fn over(ended: &mut tokio::sync::watch::Receiver<bool>) {
    while !*ended.borrow_and_update() {
        if ended.changed().await.is_err() {
            return;
        }
    }
}

/// One channel, answered: the tool's run activated for the exchange
/// — its secrets taken and fed, see
/// [`activate_tool`](crate::containers::activate_tool) — which
/// failing is the exchange's error; else the exchange put to the tool
/// stamped with `caller`, this daemon's word on who is asking, its
/// answer attested as this daemon's own tool, sent as one channel
/// response and the finish — or, for the notifications, which carry
/// no request and activate nothing, one per notification until the
/// tool's stream ends. A disconnect never reaches here. Every
/// exchange is a use of the tool, in flight into it from before its
/// activation until it is answered. A call and a read that reach the
/// tool are on the graph, each one event from `sender` — the account
/// the connect is served for, as a client — to the tool, however the
/// tool answers; one the activation refused is not, nor is a listing.
async fn answer(daemon: Arc<Daemon>, scope: Arc<ScopeHandle>, run: Arc<ToolRun>, caller: Arc<Caller>, sender: Arc<Creator>, channel: u32, ask: channel_request::Frame) {
    run.touch();
    let _flying = run.inflight.enter();
    let activated = match &ask {
        channel_request::Frame::Disconnect | channel_request::Frame::McpNotifications(_) => Ok(()),
        _ => containers::activate_tool(&daemon, &run)
            .await
            .map_err(|error| ErrorData::internal_error(error.0.to_string(), None)),
    };
    match ask {
        channel_request::Frame::Disconnect => {}
        channel_request::Frame::McpListTools(request) => {
            let frame = match activated {
                Err(error) => mcp::list_tools::response::Frame::Error(error),
                Ok(()) => {
                    let mut params = request.0.unwrap_or_default();
                    attest_request(&mut params, &*caller);
                    match run.handle.list_tools(Some(params)).await {
                        Ok(mut result) => {
                            for tool in &mut result.tools {
                                run.attest(tool.meta.get_or_insert_default());
                            }
                            mcp::list_tools::response::Frame::Result(result)
                        }
                        Err(error) => mcp::list_tools::response::Frame::Error(ErrorData::internal_error(error, None)),
                    }
                }
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpListResources(request) => {
            let frame = match activated {
                Err(error) => mcp::list_resources::response::Frame::Error(error),
                Ok(()) => {
                    let mut params = request.0.unwrap_or_default();
                    attest_request(&mut params, &*caller);
                    match run.handle.list_resources(Some(params)).await {
                        Ok(mut result) => {
                            for resource in &mut result.resources {
                                run.attest(resource.meta.get_or_insert_default());
                            }
                            mcp::list_resources::response::Frame::Result(result)
                        }
                        Err(error) => mcp::list_resources::response::Frame::Error(ErrorData::internal_error(error, None)),
                    }
                }
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpCallTool(request) => {
            let frame = match activated {
                Err(error) => mcp::call_tool::response::Frame::Error(error),
                Ok(()) => {
                    let mut params = request.0;
                    attest_request(&mut params, &*caller);
                    containers::events::record(&daemon, &sender, &run.sender, Action::Call).await;
                    match run.handle.call_tool(params).await {
                        Ok(mut result) => {
                            run.attest(result.meta.get_or_insert_default());
                            mcp::call_tool::response::Frame::Result(result)
                        }
                        Err(error) => mcp::call_tool::response::Frame::Error(ErrorData::internal_error(error, None)),
                    }
                }
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpReadResource(request) => {
            let frame = match activated {
                Err(error) => mcp::read_resource::response::Frame::Error(error),
                Ok(()) => {
                    let mut params = request.0;
                    attest_request(&mut params, &*caller);
                    containers::events::record(&daemon, &sender, &run.sender, Action::Read).await;
                    match run.handle.read_resource(params).await {
                        Ok(mut result) => {
                            run.attest(result.meta.get_or_insert_default());
                            mcp::read_resource::response::Frame::Result(result)
                        }
                        Err(error) => mcp::read_resource::response::Frame::Error(ErrorData::internal_error(error, None)),
                    }
                }
            };
            respond(&scope, channel, &frame).await;
        }
        channel_request::Frame::McpNotifications(_) => {
            match run.handle.notifications().await {
                Ok(mut stream) => {
                    while let Some(item) = stream.next().await {
                        let frame = match item {
                            Ok(mut notification) => {
                                run.attest(notification.get_meta_mut());
                                mcp::notifications::response::Frame::Notification(notification)
                            }
                            Err(error) => mcp::notifications::response::Frame::Error(ErrorData::internal_error(format!("{error:?}"), None)),
                        };
                        let last = matches!(frame, mcp::notifications::response::Frame::Error(_));
                        send(&scope, channel, &frame).await;
                        if last {
                            break;
                        }
                    }
                }
                Err(error) => {
                    send(&scope, channel, &mcp::notifications::response::Frame::Error(ErrorData::internal_error(error, None))).await;
                }
            }
            scope.send_channel_response_finish(channel).await;
        }
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
