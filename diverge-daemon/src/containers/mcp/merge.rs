//! The five MCP exchanges over a run's served tools.
//!
//! Every exchange put to a served tool activates the tool's run first
//! — its secrets taken and fed, the proxy's gate opened, see
//! [`activate_tool`](crate::containers::activate_tool) — and is in
//! flight into it from before that activation to its answer, so an
//! inactive that crosses it finds the run busy and activates it
//! again. An activation that fails is the exchange's error — a list
//! that lists nothing from that tool, a call or a read refused.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::{Stream, StreamExt as _, stream};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::tool_log::{self as log, Item};
use diverge_sdk::shared::mcp::attest_request;
use diverge_sdk::shared::mcp::attestation;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, GetMeta as _, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};
use tokio::sync::{Mutex, mpsc};
use tokio::task::AbortHandle;

use super::{Entry, Served, exposed, split};
use crate::containers::{Key, ToolKey, ToolRun, activate_tool, events, tool_log, tools};
use crate::daemon::Daemon;
use crate::store::DependencyToolId;
use crate::store::tools::Tool;

/// The notifications a container hears: every served tool's, and the
/// word that the set changed.
pub type Notifications = Pin<Box<dyn Stream<Item = Result<ServerNotification, ErrorData>> + Send>>;

/// Who is asking, and whose tools: the daemon, the container asking,
/// and its served set.
pub struct Context {
    /// The daemon.
    pub daemon: Arc<Daemon>,
    /// The container.
    pub user: Key,
    /// The agent the container is, as the daemon attests it on what
    /// it sends outward; none for a tool container, which sends
    /// nothing outward.
    pub caller: Option<attestation::Caller>,
    /// The container as a sender: who its calls and reads are from,
    /// as the graph records them.
    pub sender: Creator,
    /// The served set.
    pub served: Arc<Mutex<Served>>,
}

/// What a served entry comes to, read under the served lock and acted
/// on after it.
enum Next {
    /// A run this set holds.
    Run(Arc<ToolRun>),
    /// An attached tool, whose run is the live set's to find.
    Attached(Tool),
    /// A service, whose run the live set holds.
    Service(DependencyToolId),
    /// Nothing is served under the key.
    Missing,
}

/// The served tool's run: a transient dependency tool's; an attached
/// tool's as the live set holds it now, started now if it holds none;
/// or a service's as the live set holds it now, which is an error
/// while the service's container is not up.
pub async fn running(context: &Context, key: ToolKey) -> Result<Arc<ToolRun>, String> {
    let next = {
        let served = context.served.lock().await;
        match served.entry(key) {
            Some(Entry::Transient(run)) => Next::Run(Arc::clone(run)),
            Some(Entry::Attached { tool }) => Next::Attached(tool.clone()),
            Some(Entry::Service { id, .. }) => Next::Service(*id),
            None => Next::Missing,
        }
    };
    let tool = match next {
        Next::Run(run) => return Ok(run),
        Next::Service(id) => {
            return context
                .daemon
                .live
                .tool_run(ToolKey::Dependency(id))
                .await
                .ok_or_else(|| "the service is not running now".to_string());
        }
        Next::Missing => return Err("no tool is served under that prefix".to_string()),
        Next::Attached(tool) => tool,
    };
    if let Some(run) = context.daemon.live.tool_run(key).await {
        return Ok(run);
    }
    let run = tools::use_tool(&context.daemon, &tool).await.map_err(|error| error.to_string())?;
    context.served.lock().await.started();
    Ok(run)
}

/// The served tool's run as the live set holds it now, for the relay
/// of its notifications: a transient dependency tool's own, an
/// attached tool's or a service's by key; `None` while it has none.
async fn relayed(served: &Mutex<Served>, daemon: &Daemon, key: ToolKey) -> Option<Arc<ToolRun>> {
    let next = {
        let served = served.lock().await;
        match served.entry(key) {
            Some(Entry::Transient(run)) => return Some(Arc::clone(run)),
            Some(Entry::Attached { .. }) | Some(Entry::Service { .. }) => true,
            None => false,
        }
    };
    if !next {
        return None;
    }
    daemon.live.tool_run(key).await
}

/// The refusal for a call or a read with no caller: a tool container
/// calls no tool — a tool declares no dependencies, so nothing is ever
/// served to one, and no session could name it.
fn no_caller() -> ErrorData {
    ErrorData::invalid_params("a tool container calls no tool", None)
}

/// Every idle served tool started: what a loop beginning does, so its
/// calls find their containers up.
pub async fn start_all(context: &Context) {
    let keys = context.served.lock().await.keys();
    for key in keys {
        let _ = running(context, key).await;
    }
}

/// The union of every served tool's tools, each name under its
/// prefix: every tool served as of this moment asked, and every one
/// waited for. A tool that does not answer lists nothing. Each ask is
/// in flight into its tool for as long as it takes.
pub async fn list_tools(context: &Context, _: Option<PaginatedRequestParams>) -> Result<ListToolsResult, ErrorData> {
    let Some(caller) = &context.caller else {
        return Ok(ListToolsResult::with_all_items(Vec::new()));
    };
    let keys = context.served.lock().await.keys();
    let mut tools = Vec::new();
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        let Some(prefix) = context.served.lock().await.prefix(key) else {
            continue;
        };
        let mut params = PaginatedRequestParams::default();
        attest_request(&mut params, caller);
        tool_log::session(&context.daemon, &run, caller).await;
        let _flying = run.inflight.enter();
        if activate_tool(&context.daemon, &run).await.is_err() {
            continue;
        }
        if let Ok(result) = run.handle.list_tools(caller, Some(params)).await {
            run.touch();
            for mut tool in result.tools {
                tool.name = exposed(&prefix, &tool.name).into();
                run.attest(tool.meta.get_or_insert_default());
                tools.push(tool);
            }
        }
    }
    Ok(ListToolsResult::with_all_items(tools))
}

/// The union of every served tool's resources, verbatim.
pub async fn list_resources(context: &Context, _: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, ErrorData> {
    let Some(caller) = &context.caller else {
        return Ok(ListResourcesResult::with_all_items(Vec::new()));
    };
    let keys = context.served.lock().await.keys();
    let mut resources = Vec::new();
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        let mut params = PaginatedRequestParams::default();
        attest_request(&mut params, caller);
        tool_log::session(&context.daemon, &run, caller).await;
        let _flying = run.inflight.enter();
        if activate_tool(&context.daemon, &run).await.is_err() {
            continue;
        }
        if let Ok(result) = run.handle.list_resources(caller, Some(params)).await {
            run.touch();
            for mut resource in result.resources {
                run.attest(resource.meta.get_or_insert_default());
                resources.push(resource);
            }
        }
    }
    Ok(ListResourcesResult::with_all_items(resources))
}

/// The call, to the tool its prefix names, by the name that tool
/// knows: in flight into the tool from the ask to the answer, which
/// holds the tool's run up — a detach meanwhile serves the tool no
/// more from that instant, and this call is answered still. On the
/// graph once it reaches the run: one event from the container
/// to the tool, however the tool answers. On the tool's log, an
/// agent's call is kept once its tool is found, and its answer — the
/// result, or why there was none — once there is one, as
/// [`tool_log`] states; a tool container's own call keeps nothing.
pub async fn call_tool(context: &Context, mut params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
    let Some((prefix, name)) = split(&params.name) else {
        return Err(ErrorData::invalid_params(format!("no served tool is named {}", params.name), None));
    };
    let Some(key) = context.served.lock().await.owner(prefix) else {
        return Err(ErrorData::invalid_params(format!("no served tool is named {}", params.name), None));
    };
    let Some(caller) = &context.caller else {
        return Err(no_caller());
    };
    let name = name.to_string();
    let call = log::Call { caller: caller.clone(), name: name.clone(), arguments: params.arguments.clone() };
    let ask = tool_log::begin(&context.daemon, key, Item::Call(call)).await;
    params.name = name.into();
    let answered = routed_call(context, caller, key, params).await;
    tool_log::end(&context.daemon, key, ask, |call| match &answered {
        Ok(result) => Item::CallResult(log::CallResult { call, result: result.clone() }),
        Err(error) => Item::CallError(log::CallError { call, error: tool_log::error_data(error) }),
    })
    .await;
    answered
}

/// The call, routed: the tool's run found, the call attested, the run
/// activated, the event recorded, and the tool asked on the caller's
/// session.
async fn routed_call(context: &Context, caller: &attestation::Caller, key: ToolKey, mut params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
    let run = running(context, key).await.map_err(|error| ErrorData::internal_error(error, None))?;
    attest_request(&mut params, caller);
    tool_log::session(&context.daemon, &run, caller).await;
    run.touch();
    let _flying = run.inflight.enter();
    activate_tool(&context.daemon, &run)
        .await
        .map_err(|error| ErrorData::internal_error(error.0.to_string(), None))?;
    events::record(&context.daemon, &context.sender, &run.sender).await;
    let mut result = run
        .handle
        .call_tool(caller, params)
        .await
        .map_err(|error| ErrorData::internal_error(error, None))?;
    run.attest(result.meta.get_or_insert_default());
    Ok(result)
}

/// The read, from the first served tool that answers it: on the graph
/// as one event from the container to that tool, and to no
/// tool that did not. On each running tool's log, an agent's read is
/// kept as it is asked, and its answer — the result, or why that tool
/// had none — as it comes, as [`tool_log`] states; a tool container's
/// own read keeps nothing.
pub async fn read_resource(context: &Context, mut params: ReadResourceRequestParams) -> Result<ReadResourceResult, ErrorData> {
    let Some(caller) = &context.caller else {
        return Err(no_caller());
    };
    attest_request(&mut params, caller);
    let keys = context.served.lock().await.keys();
    let mut last = None;
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        tool_log::session(&context.daemon, &run, caller).await;
        let read = log::Read { caller: caller.clone(), uri: params.uri.clone() };
        let ask = tool_log::begin(&context.daemon, key, Item::Read(read)).await;
        let _flying = run.inflight.enter();
        if let Err(error) = activate_tool(&context.daemon, &run).await {
            let error = error.0.to_string();
            tool_log::end(&context.daemon, key, ask, |read| Item::ReadError(log::ReadError { read, error: tool_log::words(&error) })).await;
            last = Some(error);
            continue;
        }
        match run.handle.read_resource(caller, params.clone()).await {
            Ok(mut result) => {
                run.touch();
                run.attest(result.meta.get_or_insert_default());
                events::record(&context.daemon, &context.sender, &run.sender).await;
                tool_log::end(&context.daemon, key, ask, |read| Item::ReadResult(log::ReadResult { read, result: result.clone() })).await;
                return Ok(result);
            }
            Err(error) => {
                tool_log::end(&context.daemon, key, ask, |read| Item::ReadError(log::ReadError { read, error: tool_log::words(&error) })).await;
                last = Some(error);
            }
        }
    }
    Err(ErrorData::invalid_params(
        last.unwrap_or_else(|| format!("no served tool holds {}", params.uri)),
        None,
    ))
}

/// Every served tool's notifications, forwarded for as long as the
/// set serves it and its run lives, and the set's own word that it
/// changed: a relay per served tool, started when the tool is served
/// and its run is up, ended when it leaves the set or its run ends,
/// kept in step at every change of the set and every start of a
/// served tool's run — so a tool attached while the stream is open
/// has its notifications forwarded too, a tool list changed among
/// them, and a tool detached has them forwarded no more.
pub async fn notifications(context: &Context) -> Notifications {
    let (sender, receiver) = mpsc::unbounded_channel::<Result<ServerNotification, ErrorData>>();
    let (mut changes, mut generation) = {
        let served = context.served.lock().await;
        (served.subscribe(), served.generation())
    };
    {
        let sender = sender.clone();
        tokio::spawn(async move {
            while let Ok(notification) = changes.recv().await {
                if sender.send(Ok(notification)).is_err() {
                    break;
                }
            }
        });
    }
    // A tool container's context has no caller, and nothing served.
    let Some(caller) = context.caller.clone() else {
        return Box::pin(stream::unfold(receiver, |mut receiver| async move {
            let item = receiver.recv().await?;
            Some((item, receiver))
        }));
    };
    let daemon = Arc::clone(&context.daemon);
    let served = Arc::clone(&context.served);
    tokio::spawn(async move {
        let mut relays: HashMap<ToolKey, AbortHandle> = HashMap::new();
        loop {
            if sender.is_closed() {
                break;
            }
            let keys = served.lock().await.keys();
            relays.retain(|key, relay| {
                let kept = keys.contains(key) && !relay.is_finished();
                if !kept {
                    relay.abort();
                }
                kept
            });
            for key in keys {
                if relays.contains_key(&key) {
                    continue;
                }
                let Some(run) = relayed(&served, &daemon, key).await else {
                    continue;
                };
                tool_log::session(&daemon, &run, &caller).await;
                let Ok(mut stream) = run.handle.notifications(&caller).await else {
                    continue;
                };
                let sender = sender.clone();
                let relay = tokio::spawn(async move {
                    while let Some(item) = stream.next().await {
                        let forwarded = match item {
                            Ok(mut notification) => {
                                run.attest(notification.get_meta_mut());
                                Ok(notification)
                            }
                            Err(error) => Err(ErrorData::internal_error(format!("{error:?}"), None)),
                        };
                        if sender.send(forwarded).is_err() {
                            break;
                        }
                    }
                });
                relays.insert(key, relay.abort_handle());
            }
            if generation.changed().await.is_err() {
                break;
            }
        }
        for relay in relays.into_values() {
            relay.abort();
        }
    });
    Box::pin(stream::unfold(receiver, |mut receiver| async move {
        let item = receiver.recv().await?;
        Some((item, receiver))
    }))
}
