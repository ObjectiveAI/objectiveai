//! Deploying one container's dependencies.

use std::sync::Arc;

use diverge_sdk::daemon::creator;
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::tools::Tool as Declared;
use diverge_sdk::shared::error::Error;
use rmcp::model::ContentBlock;
use tokio::sync::Mutex;

use super::{position, template_of};
use crate::containers::mcp::Served;
use crate::containers::{Key, message, start, tools};
use crate::daemon::Daemon;
use crate::store::tools::{Tool, attachments};
use crate::store::{self, AgentId, agents, routes, tools as tool_records, tools_templates};

/// What a deploy is for: the container, the chain its run is on, the
/// deployer it may ask, and the served set the answers land in.
pub struct Deploy {
    /// The daemon.
    pub daemon: Arc<Daemon>,
    /// The container whose dependencies these are.
    pub user: Key,
    /// The container as a sender.
    pub sender: creator::Creator,
    /// The root of the chain, by name, if it has one.
    pub root: Option<String>,
    /// The templates down the chain so far.
    pub chain: Vec<String>,
    /// The deployer agent as the record names it.
    pub deployer: Option<creator::Agent>,
    /// Where answered dependencies are served.
    pub served: Arc<Mutex<Served>>,
}

/// Answer every declared dependency, in order; the first unmet is the
/// failure, in its own words, and the run does not go on.
pub async fn deploy(deploy: &Deploy, declared: Vec<Declared>) -> Result<(), Error> {
    for dependency in declared {
        let tool = answer(deploy, &dependency).await.map_err(|why| {
            Error(serde_json::json!({
                "kind": "dependency",
                "tool": dependency.name,
                "error": why,
            }))
        })?;
        let chain = {
            let mut chain = deploy.chain.clone();
            if let Some(template) = tool.template() {
                chain.push(template.to_string());
            }
            chain
        };
        let run = tools::use_tool(&deploy.daemon, &tool, deploy.user, deploy.root.clone(), chain)
            .await
            .map_err(|error| {
                Error(serde_json::json!({
                    "kind": "dependency",
                    "tool": dependency.name,
                    "error": error.to_string(),
                }))
            })?;
        let mut served = deploy.served.lock().await;
        served.insert(tool.clone(), &dependency.name);
        served.running(tool.id, run);
    }
    Ok(())
}

/// The tool that answers one dependency: by a route, by an
/// attachment at the root, or by the deployer; else why not.
async fn answer(deploy: &Deploy, dependency: &Declared) -> Result<Tool, String> {
    let template = template_of(dependency).map_err(|error| error.to_string())?;
    let mut conn = deploy.daemon.store.acquire().await.map_err(|error| error.to_string())?;
    if tools_templates::by_id(&mut conn, &template, false)
        .await
        .map_err(|error| error.to_string())?
        .is_none()
    {
        return Err(format!("the dependency names no tool template on record: {template}"));
    }
    let path = position(deploy.root.as_deref(), &deploy.chain, &template);
    if let Some(tool) = answered(&mut conn, deploy, path.as_ref(), &template)
        .await
        .map_err(|error| error.to_string())?
    {
        return Ok(tool);
    }
    drop(conn);
    let Some(deployer) = &deploy.deployer else {
        return Err("no route answers the position, and the container has no deployer agent".to_string());
    };
    ask_deployer(deploy, deployer, dependency, path.as_ref(), &template).await
}

/// Whether the position is answered now: a route there whose tool is
/// of the template, or — at the chain's root — a tool of the template
/// attached to the agent.
async fn answered(conn: &mut sqlx::PgConnection, deploy: &Deploy, path: Option<&Path>, template: &str) -> Result<Option<Tool>, store::Error> {
    if let Some(path) = path
        && let Some(route) = routes::by_path(conn, path, false).await?
        && let Some(tool) = tool_records::by_id(conn, route.tool, false).await?
        && tool.template() == Some(template)
    {
        return Ok(Some(tool));
    }
    if deploy.chain.is_empty()
        && let Key::Agent(agent) = deploy.user
    {
        for id in attachments::of_agent(conn, agent).await? {
            if let Some(tool) = tool_records::by_id(conn, id, false).await?
                && tool.template() == Some(template)
            {
                return Ok(Some(tool));
            }
        }
    }
    Ok(None)
}

/// The deployer asked, in its queue, and waited on: the position
/// answered first is the tool; the deployer inactive first is the
/// dependency unmet.
async fn ask_deployer(deploy: &Deploy, deployer: &creator::Agent, dependency: &Declared, path: Option<&Path>, template: &str) -> Result<Tool, String> {
    let reference = reference::Agent::TemplateIndex {
        template: deployer.template.clone(),
        index: deployer.index,
    };
    let agent = {
        let mut conn = deploy.daemon.store.acquire().await.map_err(|error| error.to_string())?;
        agents::by_reference(&mut conn, &reference, false)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "the deployer agent named is none the daemon has".to_string())?
    };
    let queue = deploy.daemon.live.deployer_queue(agent.id).await;
    let _turn = queue.lock().await;
    let run = start::agent(&deploy.daemon, &agent).await.map_err(|error| error.to_string())?;
    let mut loop_active = run.loop_active.subscribe();
    let notified = deploy.daemon.live.answered.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    let content = vec![ContentBlock::text(
        serde_json::json!({
            "dependency": {
                "name": dependency.name,
                "template": template,
                "instructions": dependency.instructions,
                "position": path,
            },
            "for": deploy.sender,
        })
        .to_string(),
    )];
    match message::enqueue(&run, content, deploy.sender.clone()).await {
        message::Fate::Delivered => {}
        message::Fate::Cancelled => return Err("the deployer did not take the dependency".to_string()),
        message::Fate::Error(error) => return Err(format!("the deployer did not take the dependency: {}", error.0)),
    }
    // The loop took the message; wait for it to begin, then to end,
    // and look for the answer at every wake.
    let _ = loop_active.wait_for(|active| *active).await;
    loop {
        {
            let mut conn = deploy.daemon.store.acquire().await.map_err(|error| error.to_string())?;
            if let Some(tool) = answered(&mut conn, deploy, path, template).await.map_err(|error| error.to_string())? {
                return Ok(tool);
            }
        }
        if !*loop_active.borrow() {
            return Err("the deployer went inactive without answering the position".to_string());
        }
        tokio::select! {
            () = notified.as_mut() => {
                notified.set(deploy.daemon.live.answered.notified());
                notified.as_mut().enable();
            }
            changed = loop_active.changed() => {
                if changed.is_err() {
                    return Err("the deployer's run ended without answering the position".to_string());
                }
            }
        }
    }
}

/// The agent a deploy is for, when it is one.
pub fn agent_of(user: Key) -> Option<AgentId> {
    match user {
        Key::Agent(id) => Some(id),
        Key::Tool(_) => None,
    }
}
