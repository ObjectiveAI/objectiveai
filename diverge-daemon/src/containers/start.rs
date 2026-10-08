//! Starting a run: the container made from the record.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item, Provider};
use diverge_sdk::provider::endpoints::containers::agents::run::client::{execute as agents_run, request as agents_request};
use diverge_sdk::provider::endpoints::containers::tools::connect::client::{execute as tools_connect, request as connect_request};
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use diverge_sdk::shared::containers::request::Connect;
use tokio::sync::{Mutex, watch};

use super::answerers::Answerer;
use super::fuse::Mounts;
use super::mcp::Served;
use super::{AgentRun, Caller, Key, StartError, ToolHandle, ToolRun, build, idle, provider, pump, sender_of_agent, sender_of_tool, serve_name, stop};
use crate::daemon::{Daemon, Kind};
use crate::store::agents::Agent;
use crate::store::tools::{Origin, Tool, attachments};
use crate::store::{agents, agents_templates, tools as tool_records, tools_templates};

/// The agent's run, started now if it was not up: the provider
/// chosen, the mounts served, the attached tools served idle, the run
/// opened and its id taken, the log told it began, the pump and the
/// idle clock running. A start that fails keeps an `Error` item in
/// the log, and the mounts opened are let go.
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
    let (identity, handle) = provider::choose(daemon, agent.provider.as_ref().map(|provider| &provider.identity))
        .await
        .map_err(StartError::Provider)?;
    let mounts = Arc::new(
        Mounts::build(
            daemon,
            Key::Agent(agent.id),
            &template.template.fuse_file_mounts,
            &template.template.fuse_directory_mounts,
            &agent.fuse_file_mounts,
            &agent.fuse_directory_mounts,
        )
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
    let answerer = Arc::new(Answerer {
        daemon: Arc::clone(daemon),
        key: Key::Agent(agent.id),
        caller: Caller::Agent {
            key: agent.key(),
            image: image.clone(),
        },
        sender: sender_of_agent(agent),
        account: agent.account,
        root: agent.name.clone(),
        chain: Vec::new(),
        deployer: agent.deployer.clone(),
        mounts: Arc::clone(&mounts),
        served: Arc::clone(&served),
        touched: touched.clone(),
    });
    let opened = agents_run::execute(&handle, &agents_request::Frame(container), answerer.set()).await;
    let (id, handle, stream) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            mounts.stop().await;
            return Err(StartError::Run(format!("{error:?}")));
        }
    };
    let run = Arc::new(AgentRun {
        id: agent.id,
        name: agent.name.clone(),
        key: agent.key(),
        image,
        sender: sender_of_agent(agent),
        account: agent.account,
        provider: identity.clone(),
        container: id.id,
        handle,
        loop_active: watch::channel(false).0,
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
    Ok(run)
}

/// The tool's run, started now: a created tool's container run on a
/// provider, with its mounts served and its own dependencies
/// deployable; a connected tool's container joined through its
/// provider. A waiter takes the run down when it ends. The chain is
/// the one the first user's run is on.
pub async fn tool(daemon: &Arc<Daemon>, tool: &Tool, root: Option<String>, chain: Vec<String>) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(tool.id).await {
        return Ok(run);
    }
    let served = Arc::new(Mutex::new(Served::new()));
    let touched = watch::channel(Instant::now()).0;
    let (identity, container, handle, mounts, image) = match &tool.origin {
        Origin::Created { template, provider: pinned } => {
            let template = {
                let mut conn = daemon.store.acquire().await?;
                tools_templates::by_id(&mut conn, template, false)
                    .await?
                    .ok_or_else(|| StartError::NoTemplate(template.clone()))?
            };
            let (identity, handle) = provider::choose(daemon, pinned.as_ref().map(|provider| &provider.identity))
                .await
                .map_err(StartError::Provider)?;
            let mounts = Arc::new(
                Mounts::build(
                    daemon,
                    Key::Tool(tool.id),
                    &template.template.fuse_file_mounts,
                    &template.template.fuse_directory_mounts,
                    &tool.fuse_file_mounts,
                    &tool.fuse_directory_mounts,
                )
                .await
                .map_err(StartError::Mounts)?,
            );
            let volumes = pinned.as_ref().map(|provider| provider.volume_mounts.as_slice()).unwrap_or(&[]);
            let container = build::container(&template.template, volumes, &mounts);
            let image = container.image.clone();
            let answerer = Arc::new(Answerer {
                daemon: Arc::clone(daemon),
                key: Key::Tool(tool.id),
                caller: Caller::Tool {
                    key: tool.key(),
                    image: Some(image.clone()),
                },
                sender: sender_of_tool(tool),
                account: tool.account,
                root,
                chain,
                deployer: tool.deployer.clone(),
                mounts: Arc::clone(&mounts),
                served: Arc::clone(&served),
                touched: touched.clone(),
            });
            match tools_run::execute(&handle, &tools_request::Frame(container), answerer.set()).await {
                Ok((id, handle)) => (identity, Some(id.id), ToolHandle::Run(handle), mounts, Some(image)),
                Err(error) => {
                    mounts.stop().await;
                    return Err(StartError::Run(format!("{error:?}")));
                }
            }
        }
        Origin::Connected { provider, id, authorization } => {
            let Some(handle) = daemon.live.provider(provider).await else {
                return Err(StartError::Provider(provider::NoProvider::NotConnected(provider.clone())));
            };
            let request = connect_request::Frame(Connect {
                id: id.clone(),
                authorization: authorization.clone(),
            });
            let joined = tools_connect::execute(&handle, &request)
                .await
                .map_err(|error| StartError::Run(format!("{error:?}")))?;
            let mounts = Arc::new(Mounts::empty());
            (provider.clone(), None, ToolHandle::Joined(joined), mounts, None)
        }
    };
    let run = Arc::new(ToolRun {
        id: tool.id,
        key: tool.key(),
        image,
        sender: sender_of_tool(tool),
        account: tool.account,
        provider: identity.clone(),
        container,
        handle,
        users: Mutex::new(HashSet::new()),
        touched,
        mounts,
        volumes: crate::volumes::of_tool(tool),
        served,
        tasks: Mutex::new(Vec::new()),
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
    run.tasks.lock().await.push(waiter.abort_handle());
    Ok(run)
}
