//! The five MCP exchanges over a run's served tools.

use std::pin::Pin;
use std::sync::Arc;

use futures_util::{Stream, StreamExt as _, stream};
use diverge_sdk::shared::mcp::attest_request;
use diverge_sdk::shared::mcp::attestation::Attestation;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, GetMeta as _, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};
use tokio::sync::{Mutex, mpsc};

use super::{Entry, Served, exposed, split};
use crate::containers::{Key, ToolKey, ToolRun, User, tools};
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
    /// Who the container is, as the daemon attests it.
    pub caller: Attestation,
    /// The served set.
    pub served: Arc<Mutex<Served>>,
}

/// What a served entry comes to, read under the served lock and acted
/// on after it.
enum Next {
    /// A run this set holds.
    Run(Arc<ToolRun>),
    /// An attached tool with no run.
    Idle(Tool),
    /// A service, whose run the live set holds.
    Service(DependencyToolId),
    /// Nothing is served under the key.
    Missing,
}

/// The served tool's run: a transient dependency tool's; an attached
/// tool's, started now if it was idle; or a service's as the live set
/// holds it now, which is an error while the service's container is
/// not up.
pub async fn running(context: &Context, key: ToolKey) -> Result<Arc<ToolRun>, String> {
    let next = {
        let served = context.served.lock().await;
        match served.entry(key) {
            Some(Entry::Transient(run)) => Next::Run(Arc::clone(run)),
            Some(Entry::Attached { run: Some(run), .. }) => Next::Run(Arc::clone(run)),
            Some(Entry::Attached { tool, run: None }) => Next::Idle(tool.clone()),
            Some(Entry::Service { id, .. }) => Next::Service(*id),
            None => Next::Missing,
        }
    };
    let idle = match next {
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
        Next::Idle(tool) => tool,
    };
    let run = tools::use_tool(&context.daemon, &idle, User::Container(context.user))
        .await
        .map_err(|error| error.to_string())?;
    context.served.lock().await.running(key, Arc::clone(&run));
    Ok(run)
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
/// prefix. A tool that does not answer lists nothing.
pub async fn list_tools(context: &Context, _: Option<PaginatedRequestParams>) -> Result<ListToolsResult, ErrorData> {
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
        attest_request(&mut params, &context.caller);
        if let Ok(result) = run.handle.list_tools(Some(params)).await {
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
    let keys = context.served.lock().await.keys();
    let mut resources = Vec::new();
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        let mut params = PaginatedRequestParams::default();
        attest_request(&mut params, &context.caller);
        if let Ok(result) = run.handle.list_resources(Some(params)).await {
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
/// knows.
pub async fn call_tool(context: &Context, mut params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
    let Some((prefix, name)) = split(&params.name) else {
        return Err(ErrorData::invalid_params(format!("no served tool is named {}", params.name), None));
    };
    let Some(key) = context.served.lock().await.owner(prefix) else {
        return Err(ErrorData::invalid_params(format!("no served tool is named {}", params.name), None));
    };
    let name = name.to_string();
    let run = running(context, key).await.map_err(|error| ErrorData::internal_error(error, None))?;
    params.name = name.into();
    attest_request(&mut params, &context.caller);
    run.touch();
    let mut result = run
        .handle
        .call_tool(params)
        .await
        .map_err(|error| ErrorData::internal_error(error, None))?;
    run.attest(result.meta.get_or_insert_default());
    Ok(result)
}

/// The read, from the first served tool that answers it.
pub async fn read_resource(context: &Context, mut params: ReadResourceRequestParams) -> Result<ReadResourceResult, ErrorData> {
    attest_request(&mut params, &context.caller);
    let keys = context.served.lock().await.keys();
    let mut last = None;
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        match run.handle.read_resource(params.clone()).await {
            Ok(mut result) => {
                run.touch();
                run.attest(result.meta.get_or_insert_default());
                return Ok(result);
            }
            Err(error) => last = Some(error),
        }
    }
    Err(ErrorData::invalid_params(
        last.unwrap_or_else(|| format!("no served tool holds {}", params.uri)),
        None,
    ))
}

/// Every served tool's notifications, forwarded for as long as its
/// scope lives, and the set's own changes.
pub async fn notifications(context: &Context) -> Notifications {
    let (sender, receiver) = mpsc::unbounded_channel::<Result<ServerNotification, ErrorData>>();
    let (mut runs, services, mut changes) = {
        let served = context.served.lock().await;
        (served.runs(), served.services(), served.subscribe())
    };
    for id in services {
        if let Some(run) = context.daemon.live.tool_run(ToolKey::Dependency(id)).await {
            runs.push(run);
        }
    }
    for run in runs {
        if let Ok(mut stream) = run.handle.notifications().await {
            let sender = sender.clone();
            tokio::spawn(async move {
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
        }
    }
    tokio::spawn(async move {
        while let Ok(notification) = changes.recv().await {
            if sender.send(Ok(notification)).is_err() {
                break;
            }
        }
    });
    Box::pin(stream::unfold(receiver, |mut receiver| async move {
        let item = receiver.recv().await?;
        Some((item, receiver))
    }))
}
