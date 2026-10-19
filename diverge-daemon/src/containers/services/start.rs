//! One run of a service's container.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::shared::mcp::attestation;
use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::daemon::grant::Converted;
use diverge_sdk::provider::endpoints::containers::tools::run::client::{execute as tools_run, request as tools_request};
use diverge_sdk::shared::containers::dependencies::Scope;
use indexmap::IndexMap;
use tokio::sync::{Mutex, watch};

use crate::containers::answerers::Answerer;
use crate::containers::deploy::short;
use crate::containers::fuse::Mounts;
use crate::containers::capable::Needs;
use crate::containers::mcp::Served;
use crate::containers::{Inflight, Key, StartError, ToolHandle, ToolKey, ToolRun, build, provider, pump_tool, secrets};
use crate::daemon::{Daemon, Kind};
use crate::judge::{Dependent, Fixed};
use crate::store::agents;
use crate::store::dependency_tools::{DependencyTool, attachments};

/// The service's run, started now if it was not up: its container —
/// the template's image, limits and arguments, no mount, since a
/// service is many agents' and mounts nothing of any one's, and its
/// secrets bound by name through the agent that spawns the run, taken
/// at the service's own registration and held under the dependency
/// tool as holder — run on the first candidate provider that will,
/// under its template's grants with every attached agent as its
/// `self`, its run entered as live under its row with its pump. The
/// agent that spawns the run is the service's owner, for an
/// `instance` scope, and otherwise the attached agent whose
/// registration declared the template last — so the binding may
/// differ from run to run, and a vault key bound anew is taken at the
/// next. A per-agent service whose agent is gone cannot be named and
/// is not started; a service that declares secrets with no agent
/// attached is not started either, and the supervisor tries again; a
/// provider that refuses after the registration gives back what it
/// took.
pub async fn start(daemon: &Arc<Daemon>, tool: &DependencyTool) -> Result<Arc<ToolRun>, StartError> {
    let key = ToolKey::Dependency(tool.id);
    if let Some(run) = daemon.live.tool_run(key).await {
        return Ok(run);
    }
    let (owner_agent, source) = {
        let mut conn = daemon.store.acquire().await?;
        let owner_agent = match tool.owner_agent {
            Some(id) => agents::by_id(&mut conn, id, false).await?,
            None => None,
        };
        let source = if tool.declared.secrets.is_empty() {
            None
        } else {
            match tool.scope {
                Scope::Instance => owner_agent.clone(),
                Scope::Template | Scope::Global => match attachments::latest(&mut conn, tool.id).await? {
                    Some(attachment) => agents::by_id(&mut conn, attachment.agent, false).await?,
                    None => None,
                },
            }
        };
        (owner_agent, source)
    };
    let Some(owner) = tool.owner(owner_agent.as_ref()) else {
        return Err(StartError::Run("the service's agent is gone".to_string()));
    };
    let binding = match source {
        Some(source) => secrets::bind(&tool.declared.secrets, &source.secrets).map_err(StartError::Unbound)?,
        None if tool.declared.secrets.is_empty() => IndexMap::new(),
        None => return Err(StartError::Run("no agent attached to the service spawns it".to_string())),
    };
    let sender = Creator::Tool(creator::Tool::Dependency {
        template: tool.template.clone(),
        owner: owner.clone(),
    });
    let needs = Needs {
        image: Some(tool.declared.image.clone()),
        memory: tool.declared.memory,
        disk: tool.declared.disk,
        ..Needs::default()
    };
    let candidates = provider::candidates(daemon, None, &needs).await.map_err(StartError::Provider)?;
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
            caller: None,
            sender: sender.clone(),
            account: None,
            dependent: Some(dependent.clone()),
            provider: identity.clone(),
            mounts: Arc::clone(&mounts),
            served: Arc::new(Mutex::new(Served::new())),
            touched: touched.clone(),
            inflight: Arc::new(Inflight::new()),
            services: Mutex::new(None),
            declared: tool.declared.secrets.clone(),
            binding: binding.clone(),
            secrets: Mutex::new(None),
        });
        let (id, handle, stream) = match tools_run::execute(&handle, &tools_request::Frame(container.clone()), answerer.set()).await {
            Ok(opened) => opened,
            Err(error) => {
                secrets::abandon(daemon, &answerer).await;
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
            container: Some(id.id),
            handle: ToolHandle::Run(handle),
            service: true,
            connections: Mutex::new(HashSet::new()),
            held: watch::channel(false).0,
            inflight: Arc::new(Inflight::new()),
            touched: touched.clone(),
            mounts: Arc::clone(&mounts),
            volumes: Vec::new(),
            ended: watch::channel(false).0,
            tasks: Mutex::new(Vec::new()),
            edits: Mutex::new(()),
            dependency: Some(tool.clone()),
            secrets: Mutex::new(answerer.secrets.lock().await.take()),
        });
        daemon.live.insert_tool(Arc::clone(&run)).await;
        daemon.live.changed(Kind::DependencyTools);
        daemon.live.nudge_capacity(&run.provider);
        let pumping = tokio::spawn(pump_tool::pump_tool(Arc::clone(daemon), Arc::clone(&run), stream));
        run.tasks.lock().await.push(pumping.abort_handle());
        return Ok(run);
    }
    Err(StartError::Run(last))
}
