//! Deploying every dependency, at once: each on record under its
//! owner, the transient tools started, the services kept.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::shared::mcp::attestation;
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::grant::Converted;
use diverge_sdk::daemon::key;
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use diverge_sdk::shared::containers::dependencies;
use diverge_sdk::shared::containers::dependencies::{Scope, Template};
use diverge_sdk::shared::error::Error;
use futures_util::future;
use tokio::sync::{Mutex, watch};

use crate::containers::answerers::Answerer;
use crate::containers::fuse::Mounts;
use crate::containers::capable::Needs;
use crate::containers::mcp::Served;
use crate::containers::services;
use crate::containers::{Key, ToolHandle, ToolKey, ToolRun, User, build, provider, stop};
use crate::daemon::{Daemon, Kind};
use crate::judge::{Dependent, Fixed};
use crate::store::dependency_tools::{self, DependencyTool};
use crate::store::{self, AgentId, DependencyToolId, hash};

/// Deploy every template for the agent `answerer` answers for, whose
/// container runs under `id` on the answerer's provider: every
/// template's body on record, every tool of it under the agent's
/// owner — the agent itself for a transient template and an
/// `instance` service, its agent template or nobody for the other
/// scopes — made or found made, the agent attached to each as of now,
/// and detached from every tool it no longer declares, in one
/// transaction before anything starts; then every transient tool
/// started at once, each served to the agent under its template's id
/// once every one is up; then every service kept up and its hold
/// recomputed, every tool left told its attachments changed, and the
/// services served the same way. A template declared twice — one id
/// twice — a tool container asking, a store that refuses the records,
/// no provider to run on, a path of the agent's that cannot be
/// served, or a provider that will not run one — every candidate
/// tried — is the error, naming the dependency by its id where there
/// is one, and every transient tool started is stopped.
pub async fn deploy(answerer: &Answerer, id: String, templates: Vec<Template>) -> Result<(), Error> {
    let (Key::Agent(agent), Some(attestation::Caller::Agent(caller))) = (answerer.key, &answerer.caller) else {
        return Err(error("", "a tool container declares no dependencies"));
    };
    let agent_key = &caller.key;
    let creator = Creator::Agent(creator::Agent {
        template: agent_key.template.clone(),
        index: agent_key.index,
        name: agent_key.name.clone(),
    });
    let mut declared = Vec::with_capacity(templates.len());
    let mut ids = HashSet::new();
    for template in templates {
        let template_id = hash::template_id(&template.hashed()).map_err(|failure| error("", &failure.to_string()))?;
        if !ids.insert(template_id.clone()) {
            return Err(error(&template_id, "declared twice"));
        }
        declared.push((template_id, template));
    }
    let daemon = &answerer.daemon;
    let (tools, left) = record(daemon, agent, agent_key, &creator, &declared)
        .await
        .map_err(|reason| error("", &reason))?;
    let id = id.as_str();
    let started = future::join_all(tools.iter().filter(|tool| !tool.is_service()).map(|tool| async move {
        let outcome = start(answerer, agent, agent_key, id, tool).await;
        (tool.template.clone(), outcome)
    }))
    .await;
    let mut runs = Vec::with_capacity(started.len());
    let mut failed = None;
    for (template_id, outcome) in started {
        match outcome {
            Ok(run) => runs.push(run),
            Err(reason) => failed = failed.or(Some((template_id, reason))),
        }
    }
    if let Some((template_id, reason)) = failed {
        stop::stop_dependencies(daemon, runs).await;
        return Err(error(&template_id, &reason));
    }
    let mut kept = Vec::new();
    for tool in &tools {
        if tool.is_service() {
            services::keep(daemon, ToolKey::Dependency(tool.id)).await;
            services::held(daemon, ToolKey::Dependency(tool.id)).await;
            kept.push(tool.id);
        }
    }
    for id in left {
        services::held(daemon, ToolKey::Dependency(id)).await;
    }
    *answerer.services.lock().await = Some(kept);
    let mut served = answerer.served.lock().await;
    for run in runs {
        if let Some(tool) = &run.dependency {
            let serve_name = short(&tool.template);
            served.insert_transient(run, &serve_name);
        }
    }
    for tool in &tools {
        if tool.is_service() {
            served.insert_service(tool.id, &tool.template, &short(&tool.template));
        }
    }
    daemon.live.changed(Kind::DependencyTools);
    daemon.live.changed(Kind::StandaloneTools);
    Ok(())
}

/// Every declared template on record — its body, and its tool under
/// the agent's owner, made by this registration or found made — the
/// agent attached to each, as of now, and detached from every tool it
/// did not declare this time, in one transaction. The tools, in the
/// order declared, and the tools left. The reason the store refused,
/// in a sentence.
async fn record(
    daemon: &Daemon,
    agent: AgentId,
    agent_key: &key::Agent,
    creator: &Creator,
    declared: &[(String, Template)],
) -> Result<(Vec<DependencyTool>, Vec<DependencyToolId>), String> {
    let mut tx = daemon.store.begin().await.map_err(|error| error.to_string())?;
    let mut tools = Vec::with_capacity(declared.len());
    for (template_id, template) in declared {
        store::dependencies::create(&mut tx, template_id, template)
            .await
            .map_err(|error| error.to_string())?;
        let scope = match &template.mode {
            dependencies::Mode::Transient { .. } => Scope::Instance,
            dependencies::Mode::Service { scope } => *scope,
        };
        let tool = dependency_tools::find_or_create(
            &mut tx,
            &dependency_tools::Declared {
                template: template_id,
                scope,
                agent,
                agent_template: &agent_key.template,
                creator,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        dependency_tools::attachments::attach(&mut tx, tool.id, agent)
            .await
            .map_err(|error| error.to_string())?;
        tools.push(tool);
    }
    let ids: Vec<DependencyToolId> = tools.iter().map(|tool| tool.id).collect();
    let left = dependency_tools::attachments::detach_except(&mut tx, agent, &ids)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    Ok((tools, left))
}

/// One transient tool started: its mounts served from the agent, its
/// container run on the first candidate provider that will, its run
/// entered as live under its row with a waiter — a run still live
/// under the row, from an agent run that ended without it, stopped
/// first, since a row has one run. The reason it could not be, in a
/// sentence.
async fn start(answerer: &Answerer, agent: AgentId, agent_key: &key::Agent, agent_container: &str, tool: &DependencyTool) -> Result<Arc<ToolRun>, String> {
    let daemon = &answerer.daemon;
    let dependencies::Mode::Transient {
        fuse_file_mounts,
        fuse_directory_mounts,
    } = &tool.declared.mode
    else {
        return Err("a service is kept, not started here".to_string());
    };
    let key = ToolKey::Dependency(tool.id);
    if let Some(stale) = daemon.live.tool_run(key).await {
        stop::stop_dependencies(daemon, vec![stale]).await;
    }
    let needs = Needs {
        fuse_mounts: !fuse_file_mounts.is_empty() || !fuse_directory_mounts.is_empty(),
        volume_modes: Vec::new(),
        image: Some(tool.declared.image.clone()),
    };
    let candidates = provider::candidates(daemon, None, &needs).await.map_err(|error| error.to_string())?;
    let mounts = Arc::new(
        Mounts::build_dependency(daemon, &answerer.provider, agent_container, fuse_file_mounts, fuse_directory_mounts)
            .await
            .map_err(|error| error.to_string())?,
    );
    let container = build::dependency(&tool.template, &tool.declared, &mounts);
    let image = container.image.clone();
    let attestation = attestation::Tool::Dependency {
        image: image.clone(),
        template: tool.template.clone(),
        owner: key::Owner::Agent(agent_key.clone()),
    };
    let sender = Creator::Tool(creator::Tool::Dependency {
        template: tool.template.clone(),
        owner: key::Owner::Agent(agent_key.clone()),
    });
    let dependent = (!tool.declared.permissions.is_empty()).then(|| Dependent {
        fixed: Arc::new(Fixed::new(identity(agent_key, &tool.template), Converted::from(tool.declared.permissions.clone()))),
        tool: tool.id,
    });
    let touched = watch::channel(Instant::now()).0;
    let mut last = String::new();
    for (identity, handle) in candidates {
        let run_answerer = Arc::new(Answerer {
            daemon: Arc::clone(daemon),
            key: Key::Tool(key),
            caller: None,
            sender: sender.clone(),
            account: None,
            dependent: dependent.clone(),
            provider: identity.clone(),
            mounts: Arc::clone(&mounts),
            served: Arc::new(Mutex::new(Served::new())),
            touched: touched.clone(),
            inflight: Arc::clone(&answerer.inflight),
            services: Mutex::new(None),
        });
        let opened = tools_run::execute(&handle, &tools_request::Frame(container.clone()), run_answerer.set()).await;
        let (container_id, handle) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                last = format!("{error:?}");
                continue;
            }
        };
        let run = Arc::new(ToolRun {
            id: key,
            attestation: attestation.clone(),
            sender: sender.clone(),
            account: None,
            provider: identity,
            container: Some(container_id.id),
            handle: ToolHandle::Run(handle),
            service: false,
            users: Mutex::new(HashSet::from([User::Container(Key::Agent(agent))])),
            held: watch::channel(true).0,
            touched: touched.clone(),
            mounts: Arc::clone(&mounts),
            volumes: Vec::new(),
            ended: watch::channel(false).0,
            tasks: Mutex::new(Vec::new()),
            dependency: Some(tool.clone()),
        });
        daemon.live.insert_tool(Arc::clone(&run)).await;
        daemon.live.changed(Kind::DependencyTools);
        let waiter = {
            let daemon = Arc::clone(daemon);
            let run = Arc::clone(&run);
            tokio::spawn(async move {
                run.handle.wait().await;
                stop::ended_tool(&daemon, run).await;
            })
        };
        run.tasks.lock().await.push(waiter.abort_handle());
        return Ok(run);
    }
    mounts.stop().await;
    Err(format!("no provider ran it; the last said: {last}"))
}

/// The first eight characters of a template id: what a dependency is
/// served to its agent as, and known as in a standing's identity.
pub fn short(template_id: &str) -> String {
    template_id.chars().take(8).collect()
}

/// The identity a transient tool's requests are served under: the
/// agent's name, or its template's first eight digits and its index,
/// then the dependency's template, short.
fn identity(agent: &key::Agent, template_id: &str) -> String {
    let agent = match &agent.name {
        Some(name) => name.clone(),
        None => format!("{}-{}", agent.template.chars().take(8).collect::<String>(), agent.index),
    };
    format!("{agent}/{}", short(template_id))
}

/// The run's error, naming the dependency by its template's id.
fn error(template_id: &str, reason: &str) -> Error {
    Error(serde_json::json!({
        "kind": "dependency",
        "dependency": template_id,
        "error": reason,
    }))
}
