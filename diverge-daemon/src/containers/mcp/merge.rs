//! The five MCP exchanges over a run's served tools.

use std::pin::Pin;
use std::sync::Arc;

use futures_util::{Stream, StreamExt as _, stream};
use diverge_sdk::shared::mcp::{Who, attest, attest_request};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, GetMeta as _, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerNotification,
};
use tokio::sync::{Mutex, mpsc};

use super::{Entry, Served, exposed, split};
use crate::containers::{Caller, Key, ToolKey, ToolRun, User, tools};
use crate::daemon::Daemon;

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
    /// Who the container is, for the attestation.
    pub caller: Caller,
    /// The served set.
    pub served: Arc<Mutex<Served>>,
}

/// The served tool's run: a dependency's, or an attached tool's,
/// started now if it was idle.
pub async fn running(context: &Context, key: ToolKey) -> Result<Arc<ToolRun>, String> {
    let idle = {
        let served = context.served.lock().await;
        match served.entry(key) {
            Some(Entry::Dependency(run)) => return Ok(Arc::clone(run)),
            Some(Entry::Attached { run: Some(run), .. }) => return Ok(Arc::clone(run)),
            Some(Entry::Attached { tool, run: None }) => tool.clone(),
            None => return Err("no tool is served under that prefix".to_string()),
        }
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
        attest_request(&mut params, context.caller.image(), context.caller.who());
        if let Ok(result) = run.handle.list_tools(Some(params)).await {
            run.touch();
            for mut tool in result.tools {
                tool.name = exposed(&prefix, &tool.name).into();
                attest(tool.meta.get_or_insert_default(), run.image.as_ref(), Who::Tool(&run.key));
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
        attest_request(&mut params, context.caller.image(), context.caller.who());
        if let Ok(result) = run.handle.list_resources(Some(params)).await {
            run.touch();
            for mut resource in result.resources {
                attest(resource.meta.get_or_insert_default(), run.image.as_ref(), Who::Tool(&run.key));
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
    attest_request(&mut params, context.caller.image(), context.caller.who());
    run.touch();
    let mut result = run
        .handle
        .call_tool(params)
        .await
        .map_err(|error| ErrorData::internal_error(error, None))?;
    attest(result.meta.get_or_insert_default(), run.image.as_ref(), Who::Tool(&run.key));
    Ok(result)
}

/// The read, from the first served tool that answers it.
pub async fn read_resource(context: &Context, mut params: ReadResourceRequestParams) -> Result<ReadResourceResult, ErrorData> {
    attest_request(&mut params, context.caller.image(), context.caller.who());
    let keys = context.served.lock().await.keys();
    let mut last = None;
    for key in keys {
        let Ok(run) = running(context, key).await else {
            continue;
        };
        match run.handle.read_resource(params.clone()).await {
            Ok(mut result) => {
                run.touch();
                attest(result.meta.get_or_insert_default(), run.image.as_ref(), Who::Tool(&run.key));
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
    let (runs, mut changes) = {
        let served = context.served.lock().await;
        (served.runs(), served.subscribe())
    };
    for run in runs {
        if let Ok(mut stream) = run.handle.notifications().await {
            let sender = sender.clone();
            tokio::spawn(async move {
                while let Some(item) = stream.next().await {
                    let forwarded = match item {
                        Ok(mut notification) => {
                            attest(notification.get_meta_mut(), run.image.as_ref(), Who::Tool(&run.key));
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
