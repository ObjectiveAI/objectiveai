//! Starting a run: the container made from the record.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item, Provider};
use diverge_sdk::daemon::endpoints::tools::connect::client::{execute as tools_connect, request as connect_request};
use diverge_sdk::provider::endpoints::containers::agents::run::client::{execute as agents_run, request as agents_request};
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use tokio::sync::{Mutex, watch};

use super::answerers::Answerer;
use super::fuse::Mounts;
use super::mcp::Served;
use super::{AgentRun, Caller, Inflight, Key, StartError, ToolHandle, ToolKey, ToolRun, build, idle, provider, pump, sender_of_agent, sender_of_tool, serve_name, stop};
use crate::daemon::{Daemon, Kind};
use crate::daemons;
use crate::store::agents::Agent;
use crate::store::tools::{Origin, Tool, attachments};
use crate::store::{agents, agents_templates, providers_daemons, tools as tool_records, tools_templates};

/// The agent's run, started now if it was not up: the providers it
/// may run on found, the mounts served, the attached tools served
/// idle, the run opened on the first provider that runs it — its
/// dependencies deployed meanwhile, and stopped again when that
/// provider does not — and its id taken, the log told it began, the
/// pump and the idle clock running. A start that fails keeps an
/// `Error` item in the log, and the mounts opened are let go.
pub async fn agent(daemon: &Arc<Daemon>, agent: &Agent) -> Result<Arc<AgentRun>, StartError> {
    if let Some(run) = daemon.live.agent_run(agent.id).await {
        return Ok(run);
    }
    match start_agent(daemon, agent).await {
        Ok(run) => Ok(run),
        Err(error) => {
            let log = daemon.live.log(agent.id).await;
            let _held = log.lock.lock().await;
            if let Ok(wrapper) = crate::logs::append(
                &daemon.logs,
                agent.id,
                Item::Error(log::Error {
                    r#type: Default::default(),
                    error: diverge_sdk::shared::error::Error(serde_json::json!({
                        "kind": "start",
                        "error": error.to_string(),
                    })),
                }),
            )
            .await
            {
                log.latest.send_replace(wrapper.logs_index);
            }
            daemon.live.changed(Kind::Agents);
            Err(error)
        }
    }
}

async fn start_agent(daemon: &Arc<Daemon>, agent: &Agent) -> Result<Arc<AgentRun>, StartError> {
    let (template, attached) = {
        let mut conn = daemon.store.acquire().await?;
        let template = agents_templates::by_id(&mut conn, &agent.template, false)
            .await?
            .ok_or_else(|| StartError::NoTemplate(agent.template.clone()))?;
        let mut attached = Vec::new();
        for id in attachments::of_agent(&mut conn, agent.id).await? {
            if let Some(tool) = tool_records::by_id(&mut conn, id, false).await? {
                attached.push(tool);
            }
        }
        (template, attached)
    };
    let candidates = provider::candidates(daemon, agent.provider.as_ref().map(|provider| &provider.identity))
        .await
        .map_err(StartError::Provider)?;
    let mounts = Arc::new(
        Mounts::build(daemon, &agent.fuse_file_mounts, &agent.fuse_directory_mounts)
            .await
            .map_err(StartError::Mounts)?,
    );
    let volumes = agent.provider.as_ref().map(|provider| provider.volume_mounts.as_slice()).unwrap_or(&[]);
    let container = build::container(&template.template, volumes, &mounts);
    let image = container.image.clone();
    let served = Arc::new(Mutex::new(Served::new()));
    {
        let mut set = served.lock().await;
        for tool in attached {
            let name = serve_name(&tool);
            set.insert(tool, &name);
        }
    }
    let touched = watch::channel(Instant::now()).0;
    let inflight = Arc::new(Inflight::new());
    let mut last = String::new();
    for (identity, handle) in candidates {
        let answerer = Arc::new(Answerer {
            daemon: Arc::clone(daemon),
            key: Key::Agent(agent.id),
            caller: Caller::Agent {
                key: agent.key(),
                image: image.clone(),
            },
            sender: sender_of_agent(agent),
            account: agent.account,
            standing: None,
            provider: identity.clone(),
            mounts: Arc::clone(&mounts),
            served: Arc::clone(&served),
            touched: touched.clone(),
            inflight: Arc::clone(&inflight),
        });
        let opened = agents_run::execute(&handle, &agents_request::Frame(container.clone()), answerer.set()).await;
        let (id, handle, stream) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                // Dependencies deployed for this attempt go with it.
                let deployed = {
                    let mut set = served.lock().await;
                    let deployed = set.dependencies();
                    for run in &deployed {
                        set.remove(run.id);
                    }
                    deployed
                };
                stop::stop_dependencies(daemon, deployed).await;
                last = format!("{error:?}");
                continue;
            }
        };
        let run = Arc::new(AgentRun {
            id: agent.id,
            key: agent.key(),
            image,
            sender: sender_of_agent(agent),
            account: agent.account,
            provider: identity.clone(),
            container: id.id,
            handle,
            loop_active: watch::channel(false).0,
            inflight,
            touched,
            mounts,
            volumes: crate::volumes::of_agent(agent),
            served,
            messages: Mutex::new(HashMap::new()),
            tasks: Mutex::new(Vec::new()),
        });
        pump::append(
            daemon,
            &run,
            Item::Active(log::Active {
                r#type: Default::default(),
                provider: Provider { identity: identity.clone() },
            }),
        )
        .await?;
        {
            let mut conn = daemon.store.acquire().await?;
            agents::set_last(&mut conn, agent.id, &identity, Utc::now()).await?;
        }
        daemon.live.insert_agent(Arc::clone(&run)).await;
        daemon.live.changed(Kind::Agents);
        let pumping = tokio::spawn(pump::pump(Arc::clone(daemon), Arc::clone(&run), stream));
        let clock = tokio::spawn(idle::idle(Arc::clone(daemon), Arc::clone(&run)));
        run.tasks.lock().await.extend([pumping.abort_handle(), clock.abort_handle()]);
        return Ok(run);
    }
    mounts.stop().await;
    Err(StartError::Run(last))
}

/// The tool's run, started now: a created tool's container run on the
/// first of its providers that runs it, with its mounts served; a
/// connected tool connected to — the daemon named connected to, over
/// the one connection to it, and its connect opened naming the tool,
/// which starts the container there and carries the tool's MCP
/// exchanges from then on. A waiter takes the run down when it ends:
/// the container's run, or the connect scope, which the other daemon
/// finishes when the tool's run ends there and this daemon's idle
/// clock lets go after `idle_seconds` unused. A tool declares no
/// dependencies, so nothing is served to it.
pub async fn tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        return Ok(run);
    }
    let touched = watch::channel(Instant::now()).0;
    let (identity, container, handle, mounts, image) = match &tool.origin {
        Origin::Created { template, provider: pinned } => {
            let template = {
                let mut conn = daemon.store.acquire().await?;
                tools_templates::by_id(&mut conn, template, false)
                    .await?
                    .ok_or_else(|| StartError::NoTemplate(template.clone()))?
            };
            let candidates = provider::candidates(daemon, pinned.as_ref().map(|provider| &provider.identity))
                .await
                .map_err(StartError::Provider)?;
            let mounts = Arc::new(
                Mounts::build(daemon, &tool.fuse_file_mounts, &tool.fuse_directory_mounts)
                    .await
                    .map_err(StartError::Mounts)?,
            );
            let volumes = pinned.as_ref().map(|provider| provider.volume_mounts.as_slice()).unwrap_or(&[]);
            let container = build::container(&template.template, volumes, &mounts);
            let image = container.image.clone();
            let mut last = String::new();
            let mut opened = None;
            for (identity, handle) in candidates {
                let answerer = Arc::new(Answerer {
                    daemon: Arc::clone(daemon),
                    key: Key::Tool(ToolKey::Record(tool.id)),
                    caller: Caller::Tool {
                        key: tool.key(),
                        image: Some(image.clone()),
                    },
                    sender: sender_of_tool(tool),
                    account: tool.account,
                    standing: None,
                    provider: identity.clone(),
                    mounts: Arc::clone(&mounts),
                    served: Arc::new(Mutex::new(Served::new())),
                    touched: touched.clone(),
                    inflight: Arc::new(Inflight::new()),
                });
                match tools_run::execute(&handle, &tools_request::Frame(container.clone()), answerer.set()).await {
                    Ok((id, handle)) => {
                        opened = Some((identity, id.id, handle));
                        break;
                    }
                    Err(error) => last = format!("{error:?}"),
                }
            }
            let Some((identity, id, handle)) = opened else {
                mounts.stop().await;
                return Err(StartError::Run(last));
            };
            (identity, Some(id), ToolHandle::Run(handle), mounts, Some(image))
        }
        Origin::Connected { daemon: name, tool: remote } => {
            let record = {
                let mut conn = daemon.store.acquire().await?;
                providers_daemons::by_name(&mut conn, name, false)
                    .await?
                    .ok_or_else(|| StartError::NoDaemon(name.clone()))?
            };
            let peer = daemons::connect(daemon, &record).await.map_err(StartError::Daemon)?;
            let request = connect_request::Frame { tool: remote.clone() };
            let connected = tools_connect::execute(&peer.connected.handle, &request)
                .await
                .map_err(|error| StartError::Run(connect_failed(&error)))?;
            let mounts = Arc::new(Mounts::empty());
            (peer.link.provider.clone(), None, ToolHandle::Connected(connected), mounts, None)
        }
    };
    let run = Arc::new(ToolRun {
        id: ToolKey::Record(tool.id),
        key: tool.key(),
        image,
        sender: sender_of_tool(tool),
        account: tool.account,
        provider: identity.clone(),
        container,
        handle,
        users: Mutex::new(HashSet::new()),
        held: watch::channel(false).0,
        touched,
        mounts,
        volumes: crate::volumes::of_tool(tool),
        ended: watch::channel(false).0,
        tasks: Mutex::new(Vec::new()),
        dependency: None,
    });
    {
        let mut conn = daemon.store.acquire().await?;
        tool_records::set_last(&mut conn, tool.id, &identity, Utc::now()).await?;
    }
    daemon.live.insert_tool(Arc::clone(&run)).await;
    daemon.live.changed(Kind::Tools);
    let waiter = {
        let daemon = Arc::clone(daemon);
        let run = Arc::clone(&run);
        tokio::spawn(async move {
            run.handle.wait().await;
            stop::ended_tool(&daemon, run).await;
        })
    };
    let mut tasks = vec![waiter.abort_handle()];
    if let ToolHandle::Connected(_) = run.handle {
        tasks.push(tokio::spawn(idle::idle_tool(Arc::clone(daemon), Arc::clone(&run))).abort_handle());
    }
    run.tasks.lock().await.extend(tasks);
    Ok(run)
}

/// Why the other daemon's connect did not connect, in a start's words:
/// the daemon's own answers named, and this end's failures as they
/// say themselves.
fn connect_failed(error: &tools_connect::ExecuteError) -> String {
    match error {
        tools_connect::ExecuteError::NotFound => "the daemon named has no such tool".to_string(),
        tools_connect::ExecuteError::Forbidden => "the daemon named does not allow this daemon to connect to the tool".to_string(),
        tools_connect::ExecuteError::Daemon(error) => format!("the daemon named could not serve the tool: {}", error.0),
        error => format!("the connect on the daemon named failed: {error}"),
    }
}
