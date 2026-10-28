//! Starting a run: the container made from the record.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use diverge_sdk::shared::mcp::attestation;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item, Provider};
use diverge_sdk::daemon::endpoints::standalone_tools::connect::client::{execute as tools_connect, request as connect_request};
use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;
use diverge_sdk::provider::endpoints::containers::agents::run::client::{execute as agents_run, request as agents_request};
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use tokio::sync::{Mutex, watch};

use super::answerers::Answerer;
use super::capable::Needs;
use super::fuse::Mounts;
use super::mcp::Served;
use super::{AgentRun, Inflight, Key, StartError, ToolHandle, ToolKey, ToolRun, build, idle, provider, pump, pump_tool, secrets, sender_of_agent, sender_of_tool, serve_name, services, stop};
use crate::daemon::{Daemon, Kind};
use crate::daemons;
use crate::containers::permissions;
use crate::judge::ToolRef;
use crate::store::agents::Agent;
use crate::store::tools::{Origin, Tool, attachments};
use crate::store::{agents, agents_templates, tools as tool_records, tools_templates};

/// The agent's run, started now if it was not up: the providers it
/// may run on found, the mounts served, the attached tools served
/// idle, the run opened on the first provider that runs it — its
/// dependencies deployed meanwhile, the transient ones stopped again
/// when that provider does not — its dependencies settled to none
/// when its program declared none, its secrets taken at its
/// registration as [`Registrar`](diverge_sdk::provider::client::Registrar)
/// states and moved into the run, and its id taken, the log told
/// it began, the pump and the idle clock running. A start that fails
/// keeps an `Error` item in the log, the mounts opened are let go,
/// and what a registration took is given back.
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
        // The agent's own attachments first, in attach order, then its
        // template's, so the prefixes are the same at every run.
        let mut ids = attachments::of_agent(&mut conn, agent.id).await?;
        for id in tool_records::template_attachments::of_template(&mut conn, &agent.template).await? {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let mut attached = Vec::new();
        for id in ids {
            if let Some(tool) = tool_records::by_id(&mut conn, id, false).await? {
                attached.push(tool);
            }
        }
        (template, attached)
    };
    let volumes = agent.provider.as_ref().map(|provider| provider.volume_mounts.as_slice()).unwrap_or(&[]);
    let needs = Needs {
        volume_modes: volumes.iter().map(|mount| mount.volume_mode).collect(),
        image: Some(build::image(&template.template.image)),
        memory: template.template.memory,
        disk: template.template.disk,
    };
    let candidates = provider::candidates(daemon, agent.provider.as_ref().map(|provider| &provider.identity), &needs)
        .await
        .map_err(StartError::Provider)?;
    let mounts = Arc::new(
        Mounts::build(daemon, &agent.fuse_file_mounts, &agent.fuse_directory_mounts)
            .await
            .map_err(StartError::mounts)?,
    );
    let container = build::container(&agent.template, &template.template, volumes, &mounts).await;
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
            caller: Some(attestation::Caller::Agent(attestation::Agent {
                image: image.clone(),
                key: agent.key(),
            })),
            sender: sender_of_agent(agent),
            tool: None,
            permissions: Mutex::new(Vec::new()),
            provider: identity.clone(),
            mounts: Arc::clone(&mounts),
            served: Arc::clone(&served),
            touched: touched.clone(),
            inflight: Arc::clone(&inflight),
            services: Mutex::new(None),
            declared: template.template.secrets.clone(),
            binding: agent.secrets.clone(),
            secrets: Mutex::new(None),
        });
        let opened = agents_run::execute(&handle, &agents_request::Frame(container.clone()), answerer.set()).await;
        let (id, handle, stream) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                // Dependencies deployed for this attempt go with it.
                let deployed = {
                    let mut set = served.lock().await;
                    let deployed = set.transients();
                    for run in &deployed {
                        set.remove(run.id);
                    }
                    deployed
                };
                stop::stop_dependencies(daemon, deployed).await;
                secrets::abandon(daemon, &answerer).await;
                last = format!("{error:?}");
                continue;
            }
        };
        if answerer.services.lock().await.is_none() {
            services::settle(daemon, agent.id).await;
        }
        let run = Arc::new(AgentRun {
            id: agent.id,
            attestation: attestation::Caller::Agent(attestation::Agent {
                image,
                key: agent.key(),
            }),
            sender: sender_of_agent(agent),
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
            pending: Arc::new(Inflight::new()),
            secrets: Mutex::new(answerer.secrets.lock().await.take()),
            tasks: Mutex::new(Vec::new()),
            edits: Mutex::new(()),
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
        daemon.live.nudge_capacity(&run.provider);
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
/// exchanges from then on. A created tool's secrets are taken at its
/// registration, as [`Registrar`](diverge_sdk::provider::client::Registrar)
/// states, and moved into the run; [`pump_tool`](super::pump_tool)
/// reads the container's activity to its end and takes the run down;
/// a connected tool's waiter takes it down when the connect scope
/// ends, which the other daemon finishes when the tool's run ends
/// there; a transient tool's idle
/// clock ends either `idle_seconds` after the last use, nothing in
/// flight and no connect open. A tool declares no dependencies, so
/// nothing is served to it. Whether the tool is a service is its
/// template's mode, read here for the run.
pub async fn tool(daemon: &Arc<Daemon>, tool: &Tool) -> Result<Arc<ToolRun>, StartError> {
    if let Some(run) = daemon.live.tool_run(ToolKey::Record(tool.id)).await {
        return Ok(run);
    }
    let touched = watch::channel(Instant::now()).0;
    let (identity, container, handle, mounts, attestation, service, stream, secrets) = match &tool.origin {
        Origin::Created { template, provider: pinned } => {
            let record = {
                let mut conn = daemon.store.acquire().await?;
                tools_templates::by_id(&mut conn, template, false)
                    .await?
                    .ok_or_else(|| StartError::NoTemplate(template.clone()))?
            };
            let volumes = pinned.as_ref().map(|provider| provider.volume_mounts.as_slice()).unwrap_or(&[]);
            let needs = Needs {
                volume_modes: volumes.iter().map(|mount| mount.volume_mode).collect(),
                image: Some(build::image(&record.template.image)),
                memory: record.template.memory,
                disk: record.template.disk,
            };
            let candidates = provider::candidates(daemon, pinned.as_ref().map(|provider| &provider.identity), &needs)
                .await
                .map_err(StartError::Provider)?;
            let mounts = Arc::new(
                Mounts::build(daemon, &tool.fuse_file_mounts, &tool.fuse_directory_mounts)
                    .await
                    .map_err(StartError::mounts)?,
            );
            let container = build::container(template, &record.template, volumes, &mounts).await;
            let image = container.image.clone();
            let mut last = String::new();
            let mut opened = None;
            for (identity, handle) in candidates {
                let answerer = Arc::new(Answerer {
                    daemon: Arc::clone(daemon),
                    key: Key::Tool(ToolKey::Record(tool.id)),
                    caller: None,
                    sender: sender_of_tool(tool),
                    tool: Some(ToolRef::Standalone(tool.id)),
                    permissions: Mutex::new(Vec::new()),
                    provider: identity.clone(),
                    mounts: Arc::clone(&mounts),
                    served: Arc::new(Mutex::new(Served::new())),
                    touched: touched.clone(),
                    inflight: Arc::new(Inflight::new()),
                    services: Mutex::new(None),
                    declared: record.template.secrets.clone(),
                    binding: tool.secrets.clone(),
                    secrets: Mutex::new(None),
                });
                match tools_run::execute(&handle, &tools_request::Frame(container.clone()), answerer.set()).await {
                    Ok((id, handle, stream)) => {
                        let secrets = answerer.secrets.lock().await.take();
                        let permissions = answerer.permissions.lock().await.clone();
                        permissions::record(daemon, ToolRef::Standalone(tool.id), &permissions).await?;
                        opened = Some((identity, id.id, handle, stream, secrets));
                        break;
                    }
                    Err(error) => {
                        secrets::abandon(daemon, &answerer).await;
                        last = format!("{error:?}");
                    }
                }
            }
            let Some((identity, id, handle, stream, secrets)) = opened else {
                mounts.stop().await;
                return Err(StartError::Run(last));
            };
            let attestation = attestation::Tool::Created {
                image,
                template: template.clone(),
                index: tool.index,
                name: tool.name.clone(),
            };
            let service = record.template.r#type.mode == Mode::Service;
            (identity, Some(id), ToolHandle::Run(handle), mounts, attestation, service, Some(stream), secrets)
        }
        Origin::Connected { daemon: name, tool: remote } => {
            let remote_daemon = daemon.daemons.connect.iter().find(|remote| &remote.name == name).ok_or_else(|| StartError::NoDaemon(name.clone()))?;
            let peer = daemons::connect(daemon, remote_daemon).await.map_err(StartError::Daemon)?;
            let request = connect_request::Frame {
                tool: remote.clone(),
                relay: None,
            };
            let connected = tools_connect::execute(&peer.connected.handle, &request)
                .await
                .map_err(|error| StartError::Run(connect_failed(&error)))?;
            let mounts = Arc::new(Mounts::empty());
            let attestation = attestation::Tool::Connected {
                image: connected.image().clone(),
                template: connected.template().to_string(),
                index: tool.index,
                name: tool.name.clone(),
                daemon: name.clone(),
            };
            (peer.link.provider.clone(), None, ToolHandle::Connected(connected), mounts, attestation, false, None, None)
        }
    };
    let run = Arc::new(ToolRun {
        id: ToolKey::Record(tool.id),
        attestation,
        sender: sender_of_tool(tool),
        provider: identity.clone(),
        container,
        handle,
        service,
        connections: Mutex::new(HashSet::new()),
        held: watch::channel(false).0,
        inflight: Arc::new(Inflight::new()),
        touched,
        mounts,
        volumes: crate::volumes::of_tool(tool),
        ended: watch::channel(false).0,
        tasks: Mutex::new(Vec::new()),
        edits: Mutex::new(()),
        dependency: None,
        secrets: Mutex::new(secrets),
    });
    {
        let mut conn = daemon.store.acquire().await?;
        tool_records::set_last(&mut conn, tool.id, &identity, Utc::now()).await?;
    }
    daemon.live.insert_tool(Arc::clone(&run)).await;
    daemon.live.changed(Kind::StandaloneTools);
    if let ToolHandle::Run(_) = run.handle {
        daemon.live.nudge_capacity(&run.provider);
    }
    let watcher = match stream {
        Some(stream) => tokio::spawn(pump_tool::pump_tool(Arc::clone(daemon), Arc::clone(&run), stream)),
        None => {
            let daemon = Arc::clone(daemon);
            let run = Arc::clone(&run);
            tokio::spawn(async move {
                run.handle.wait().await;
                stop::ended_tool(&daemon, run).await;
            })
        }
    };
    let mut tasks = vec![watcher.abort_handle()];
    if !run.service {
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
