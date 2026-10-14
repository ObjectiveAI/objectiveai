//! One run of a service's container.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::shared::mcp::attestation::{self, Attestation};
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::grant::Converted;
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use tokio::sync::{Mutex, watch};

use crate::containers::answerers::Answerer;
use crate::containers::deploy::short;
use crate::containers::fuse::Mounts;
use crate::containers::mcp::Served;
use crate::containers::{Inflight, Key, StartError, ToolHandle, ToolKey, ToolRun, build, provider, stop};
use crate::daemon::{Daemon, Kind};
use crate::judge::{Dependent, Fixed};
use crate::store::agents;
use crate::store::dependency_tools::DependencyTool;

/// The service's run, started now if it was not up: its container —
/// the template's image, limits and arguments, no mount, since a
/// service is many agents' and mounts nothing of any one's — run on
/// the first candidate provider that will, under its template's
/// grants with every attached agent as its `self`, its run entered as
/// live under its row with a waiter. A per-agent service whose agent
/// is gone cannot be named and is not started.
pub async fn start(daemon: &Arc<Daemon>, tool: &DependencyTool) -> Result<Arc<ToolRun>, StartError> {
    let key = ToolKey::Dependency(tool.id);
    if let Some(run) = daemon.live.tool_run(key).await {
        return Ok(run);
    }
    let owner = match tool.owner_agent {
        Some(id) => {
            let mut conn = daemon.store.acquire().await?;
            agents::by_id(&mut conn, id, false).await?
        }
        None => None,
    };
    let Some(owner) = tool.owner(owner.as_ref()) else {
        return Err(StartError::Run("the service's agent is gone".to_string()));
    };
    let sender = Creator::Tool(creator::Tool::Dependency {
        template: tool.template.clone(),
        owner: owner.clone(),
    });
    let candidates = provider::candidates(daemon, None).await.map_err(StartError::Provider)?;
    let container = build::service(&tool.template, &tool.declared);
    let attestation = attestation::Tool::Dependency {
        image: container.image.clone(),
        template: tool.template.clone(),
        owner,
    };
    let dependent = Dependent {
        fixed: Arc::new(Fixed::new(
            format!("service/{}", short(&tool.template)),
            Converted::from(tool.declared.permissions.clone()),
        )),
        tool: tool.id,
    };
    let touched = watch::channel(Instant::now()).0;
    let mounts = Arc::new(Mounts::empty());
    let mut last = String::new();
    for (identity, handle) in candidates {
        let answerer = Arc::new(Answerer {
            daemon: Arc::clone(daemon),
            key: Key::Tool(key),
            caller: Attestation::Tool(attestation.clone()),
            sender: sender.clone(),
            account: None,
            dependent: Some(dependent.clone()),
            provider: identity.clone(),
            mounts: Arc::clone(&mounts),
            served: Arc::new(Mutex::new(Served::new())),
            touched: touched.clone(),
            inflight: Arc::new(Inflight::new()),
            services: Mutex::new(None),
        });
        let (id, handle) = match tools_run::execute(&handle, &tools_request::Frame(container.clone()), answerer.set()).await {
            Ok(opened) => opened,
            Err(error) => {
                last = format!("{error:?}");
                continue;
            }
        };
        let run = Arc::new(ToolRun {
            id: key,
            attestation: Attestation::Tool(attestation.clone()),
            sender: sender.clone(),
            account: None,
            provider: identity,
            container: Some(id.id),
            handle: ToolHandle::Run(handle),
            users: Mutex::new(HashSet::new()),
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
    Err(StartError::Run(last))
}
