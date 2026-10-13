//! Deploying every dependency, at once: the transient tools started,
//! the services registered.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::grant::Converted;
use diverge_sdk::daemon::key;
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use diverge_sdk::shared::containers::dependencies;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::error::Error;
use futures_util::future;
use tokio::sync::{Mutex, watch};

use crate::containers::answerers::Answerer;
use crate::containers::fuse::Mounts;
use crate::containers::mcp::Served;
use crate::containers::services::{self, Declared};
use crate::containers::{Caller, Dependency, Key, ToolHandle, ToolKey, ToolRun, User, build, provider, stop};
use crate::daemon::{Daemon, Kind};
use crate::judge::{Dependent, Fixed, Selves};
use crate::store::{self, AgentId, hash};

/// Deploy every template for the agent `answerer` answers for, whose
/// container runs under `id` on the answerer's provider: every
/// template put on record as a dependency template of the daemon's,
/// made or found made, and the agent's dependencies RESET to exactly
/// these, in one transaction before anything starts; then every
/// transient tool started at once, each served to the agent under
/// its template's id once every one is up; then every service
/// registered — found under its scope key or made, the agent attached
/// to each and detached from every service it no longer declares —
/// and served the same way. A template declared twice — one id twice
/// — a tool container asking, a store that refuses the records, no
/// provider to run on, a path of the agent's that cannot be served, a
/// provider that will not run one — every candidate tried — or a
/// registration the store refuses is the error, naming the dependency
/// by its id where there is one, and every transient tool started is
/// stopped.
pub async fn deploy(answerer: &Answerer, id: String, templates: Vec<Template>) -> Result<(), Error> {
    let (Key::Agent(agent), Caller::Agent { key: agent_key, .. }) = (answerer.key, &answerer.caller) else {
        return Err(error("", "a tool container declares no dependencies"));
    };
    let creator = Creator::Agent(creator::Agent {
        template: agent_key.template.clone(),
        index: agent_key.index,
        name: agent_key.name.clone(),
    });
    let mut transient = Vec::new();
    let mut services = Vec::new();
    let mut ids = HashSet::new();
    let mut records = Vec::with_capacity(templates.len());
    for template in templates {
        let template_id = hash::template_id(&template.hashed()).map_err(|failure| error("", &failure.to_string()))?;
        if !ids.insert(template_id.clone()) {
            return Err(error(&template_id, "declared twice"));
        }
        records.push(store::dependencies::New {
            id: template_id.clone(),
            template: template.clone(),
            creator: creator.clone(),
        });
        match &template.kind {
            dependencies::Kind::Transient { .. } => transient.push((template_id, template)),
            dependencies::Kind::Service { scope } => services.push(Declared {
                template: template_id,
                scope: *scope,
                declared: template,
            }),
        }
    }
    record(&answerer.daemon, agent, &records).await.map_err(|reason| error("", &reason))?;
    let id = id.as_str();
    let started = future::join_all(transient.into_iter().map(|(template_id, template)| async move {
        let outcome = start(answerer, agent, agent_key, id, template_id.clone(), template).await;
        (template_id, outcome)
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
        stop::stop_dependencies(&answerer.daemon, runs).await;
        return Err(error(&template_id, &reason));
    }
    let registered = match services::register(&answerer.daemon, agent, agent_key, &services).await {
        Ok(registered) => registered,
        Err(reason) => {
            stop::stop_dependencies(&answerer.daemon, runs).await;
            return Err(error("", &reason));
        }
    };
    *answerer.services.lock().await = Some(registered.clone());
    let mut served = answerer.served.lock().await;
    for run in runs {
        if let Some(dependency) = &run.dependency {
            let serve_name = short(&dependency.template);
            served.insert_dependency(run, &serve_name);
        }
    }
    for (service, declared) in registered.into_iter().zip(&services) {
        served.insert_service(service, &declared.template, &short(&declared.template));
    }
    Ok(())
}

/// Every declared template on record, made by this registration or
/// found made — a deleted one made anew, under its first creator —
/// and the agent's dependencies made exactly the templates declared,
/// in the order declared, in one transaction. The reason the store
/// refused, in a sentence.
async fn record(daemon: &Daemon, agent: AgentId, records: &[store::dependencies::New]) -> Result<(), String> {
    let mut tx = daemon.store.begin().await.map_err(|error| error.to_string())?;
    for new in records {
        store::dependencies::create(&mut tx, new).await.map_err(|error| error.to_string())?;
    }
    let ids: Vec<String> = records.iter().map(|new| new.id.clone()).collect();
    store::dependencies::agents::replace(&mut tx, agent, &ids)
        .await
        .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    daemon.live.changed(Kind::Dependencies);
    Ok(())
}

/// One transient tool started: its mounts served from the agent, its
/// container run on the first candidate provider that will, its run
/// entered as live with a waiter. The reason it could not be, in a
/// sentence.
async fn start(
    answerer: &Answerer,
    agent: AgentId,
    agent_key: &key::Agent,
    agent_container: &str,
    template_id: String,
    template: Template,
) -> Result<Arc<ToolRun>, String> {
    let daemon = &answerer.daemon;
    let dependencies::Kind::Transient {
        fuse_file_mounts,
        fuse_directory_mounts,
    } = &template.kind
    else {
        return Err("a service is registered, not started here".to_string());
    };
    let candidates = provider::candidates(daemon, None).await.map_err(|error| error.to_string())?;
    let mounts = Arc::new(Mounts::build_dependency(daemon, &answerer.provider, agent_container, fuse_file_mounts, fuse_directory_mounts).await?);
    let container = build::dependency(&template_id, &template, &mounts);
    let image = container.image.clone();
    let dependency_id = daemon.live.mint_dependency().await;
    let key = key::Tool::Dependency {
        template: template_id.clone(),
        owner: key::Owner::Agent(agent_key.clone()),
    };
    let sender = Creator::Tool(creator::Tool::Dependency {
        template: template_id.clone(),
        owner: key::Owner::Agent(agent_key.clone()),
    });
    let dependent = (!template.permissions.is_empty()).then(|| Dependent {
        fixed: Arc::new(Fixed::new(identity(agent_key, &template_id), Converted::from(template.permissions.clone()))),
        selves: Selves::Agent(agent),
    });
    let touched = watch::channel(Instant::now()).0;
    let mut last = String::new();
    for (identity, handle) in candidates {
        let run_answerer = Arc::new(Answerer {
            daemon: Arc::clone(daemon),
            key: Key::Tool(ToolKey::Dependency(dependency_id)),
            caller: Caller::Tool {
                key: key.clone(),
                image: Some(image.clone()),
            },
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
            id: ToolKey::Dependency(dependency_id),
            key: key.clone(),
            image: Some(image.clone()),
            sender: sender.clone(),
            account: None,
            provider: identity,
            container: Some(container_id.id),
            handle: ToolHandle::Run(handle),
            users: Mutex::new(HashSet::from([User::Container(Key::Agent(agent))])),
            held: watch::channel(true).0,
            touched: touched.clone(),
            mounts: Arc::clone(&mounts),
            volumes: Vec::new(),
            ended: watch::channel(false).0,
            tasks: Mutex::new(Vec::new()),
            dependency: Some(Dependency {
                id: dependency_id,
                agent,
                agent_key: agent_key.clone(),
                template: template_id,
                declared: template,
                started: Utc::now(),
            }),
            service: None,
        });
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
