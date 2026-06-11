//! `agents queue deliver` — wake every queue-pending strict
//! descendant of the caller.
//!
//! Targets come from `db::message_queue::list_delivery_targets`
//! (DISTINCT AIHs with active queued prompts in the caller's subtree,
//! direct + tag-resolved), deduped and with the caller itself
//! excluded. Per AIH, [`deliver_one`] try-acquires the agent's lock
//! file with no waiting:
//!
//! - lock held by a live owner → `AgentActive {aih}` — the agent is
//!   already running and will drain its own queue;
//! - lock won → `AgentSpawned {aih}`, then the SAME spawn machinery
//!   `agents spawn` / `agents message` use (`spawn::run_multi_pass`,
//!   empty messages, the stored continuation) streams the agent's
//!   output as `Value {aih, value}` envelopes. Ownership of the
//!   `LockClaim` transfers into the per-AIH stream's closure, so the
//!   lock is released the moment THAT task's stream ends — never held
//!   for the slowest. (`run_multi_pass`'s internal registry
//!   `observe` is best-effort and silently no-ops on the held lock.)
//!
//! Each per-AIH stream's FIRST item is always its resolution
//! (`AgentActive` / `AgentSpawned` / a setup `Err`); once every
//! target has resolved, the bare string `"AllAgentsActive"` is
//! emitted mid-stream and spawn output keeps flowing after it.
//!
//! Mode split on `dangerous_advanced.stream_spawns` (mirrors
//! `agents spawn`'s `stream`): unset/false re-execs this binary as a
//! DETACHED ORPHAN with `stream_spawns=true` and yields the child's
//! status items (spawn `Value` output is skipped) up to and including
//! `AllAgentsActive`, then returns — the orphan keeps running the
//! spawns to completion.

use std::collections::HashSet;
use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::ResponseItem as AgentsResponseItem;
use objectiveai_sdk::cli::command::agents::queue::deliver::{
    AgentActiveResponseItem, AgentActiveType, AgentSpawnedResponseItem, AgentSpawnedType,
    AllAgentsActive, Request, RequestDangerousAdvanced, ResponseItem, ValueResponseItem,
};
use objectiveai_sdk::cli::command::ResponseItem as RootResponseItem;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::db;
use crate::error::Error;
use crate::lock_file;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

/// Internal merge item: each per-AIH stream is tagged with its index
/// so the outer driver can tell when every target has resolved (each
/// stream's first item is its resolution by construction).
type TaggedStream =
    Pin<Box<dyn Stream<Item = (usize, Result<ResponseItem, Error>)> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    if request
        .dangerous_advanced
        .as_ref()
        .and_then(|adv| adv.stream_spawns)
        == Some(true)
    {
        execute_streaming(ctx, request).await
    } else {
        execute_detached(request).await
    }
}

/// Default mode: re-invoke `objectiveai-cli agents queue deliver` as
/// a detached subprocess with `stream_spawns = true`, yield the
/// child's STATUS items (`AgentActive` / `AgentSpawned` — `Value`
/// spawn output is skipped) up to and including `AllAgentsActive`,
/// and return.
/// The subprocess outlives this call — its `tokio::process::Child`
/// handle is dropped without kill (the SDK's `BinaryExecutor` default
/// + Windows `DETACHED_PROCESS` flag), so the spawns run to
/// completion as an orphan.
async fn execute_detached(request: Request) -> Result<ItemStream, Error> {
    let mut child_request = request;
    match child_request.dangerous_advanced.as_mut() {
        Some(adv) => adv.stream_spawns = Some(true),
        None => {
            child_request.dangerous_advanced = Some(RequestDangerousAdvanced {
                stream_spawns: Some(true),
            })
        }
    }

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let executor = BinaryExecutor::from_path(exe).detach(true);

    let mut stream = executor
        .execute::<Request, ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!(
            "self-respawn for agents queue deliver: {e}"
        )))?;

    let out = async_stream::stream! {
        while let Some(item) = stream.next().await {
            match item {
                // Spawn output is the child's business — detached
                // mode surfaces only the status variants.
                Ok(ResponseItem::Value(_)) => {}
                Ok(item) => {
                    let done = matches!(item, ResponseItem::AllAgentsActive(_));
                    yield Ok(item);
                    if done {
                        // Final item for the detached mode. Dropping
                        // the stream drops the Child handle without
                        // kill — the orphan finishes the spawns.
                        break;
                    }
                }
                Err(e) => yield Err(Error::Instance(format!(
                    "self-respawn for agents queue deliver: {e}"
                ))),
            }
        }
    };
    Ok(Box::pin(out))
}

/// `stream_spawns = true`: run the full delivery in-process.
async fn execute_streaming(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    let agents_dir = ctx
        .filesystem
        .base_dir()
        .join("instances")
        .join("agents");
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| Error::Instance(format!("create agents_dir: {e}")))?;

    // Unique queue-pending AIHs in the caller's subtree, caller
    // excluded (the query is parent-inclusive; deliver targets only
    // strict descendants).
    let caller = ctx.config.agent_instance_hierarchy.clone();
    let targets = db::message_queue::list_delivery_targets(ctx.db.get().await?, &caller).await?;
    let mut hierarchies: Vec<String> = Vec::new();
    for target in targets {
        if target.agent_instance_hierarchy != caller
            && !hierarchies.contains(&target.agent_instance_hierarchy)
        {
            hierarchies.push(target.agent_instance_hierarchy);
        }
    }

    let n = hierarchies.len();
    let mut select_all = futures::stream::SelectAll::new();
    for (idx, hierarchy) in hierarchies.into_iter().enumerate() {
        let tagged = deliver_one(ctx.clone(), hierarchy, agents_dir.clone())
            .map(move |item| (idx, item));
        select_all.push(Box::pin(tagged) as TaggedStream);
    }

    let out = async_stream::stream! {
        if n == 0 {
            yield Ok(ResponseItem::AllAgentsActive(AllAgentsActive::AllAgentsActive));
            return;
        }
        let mut seen: HashSet<usize> = HashSet::new();
        let mut merged = select_all;
        while let Some((idx, item)) = merged.next().await {
            let first = seen.insert(idx);
            yield item;
            // Every per-AIH stream's first item is its resolution —
            // once all have resolved, every agent is either already
            // active or freshly spawned. Spawn output keeps flowing
            // after the marker; only the detached parent stops here.
            if first && seen.len() == n {
                yield Ok(ResponseItem::AllAgentsActive(AllAgentsActive::AllAgentsActive));
            }
        }
    };
    Ok(Box::pin(out))
}

/// Deliver one AIH. The FIRST item is always the resolution:
/// `AgentActive` (lock held by a live owner), `AgentSpawned` (lock
/// won, spawn starting), or a setup `Err` (lock won but no prior
/// session). On a win, the `LockClaim`'s ownership transfers into
/// this stream (`_claim`) — released when the stream ends.
fn deliver_one(
    ctx: Context,
    hierarchy: String,
    agents_dir: std::path::PathBuf,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::stream! {
        let lock_path = agents_dir.join(hierarchy.replace('/', "_"));
        let Some(claim) = lock_file::try_acquire(&lock_path) else {
            yield Ok(ResponseItem::AgentActive(AgentActiveResponseItem {
                r#type: AgentActiveType::AgentActive,
                agent_instance_hierarchy: hierarchy,
            }));
            return;
        };
        // Lock ownership lives here for the rest of this stream;
        // dropping at stream end releases it per-AIH.
        // `run_multi_pass`'s registry observe of the same hierarchy
        // is best-effort and silently no-ops on the held lock.
        let _claim = claim;

        let lookup = match db::logs::lookup_session(ctx.db.get().await?, &hierarchy).await {
            Ok(Some(lookup)) => lookup,
            Ok(None) => {
                yield Err(Error::Instance(format!(
                    "no prior session for {hierarchy:?}"
                )));
                return;
            }
            Err(e) => {
                yield Err(e.into());
                return;
            }
        };

        yield Ok(ResponseItem::AgentSpawned(AgentSpawnedResponseItem {
            r#type: AgentSpawnedType::AgentSpawned,
            agent_instance_hierarchy: hierarchy.clone(),
        }));

        // Empty messages: the wake-up turn exists so the agent drains
        // its own queue (the conduit reads pending rows during the
        // turn), same shape `run_multi_pass` itself uses on restart.
        let params = AgentCompletionCreateParams {
            messages: Vec::new(),
            provider: None,
            agent: lookup.agent,
            response_format: None,
            seed: None,
            stream: Some(true),
            continuation: lookup.continuation,
        };
        let inner = crate::command::agents::spawn::run_multi_pass(
            ctx.clone(),
            params,
            None,
            agents_dir.clone(),
        );
        let mut inner = Box::pin(inner);
        while let Some(item) = inner.next().await {
            match item {
                Ok(spawn_item) => {
                    yield Ok(ResponseItem::Value(ValueResponseItem {
                        agent_instance_hierarchy: hierarchy.clone(),
                        value: Box::new(RootResponseItem::Agents(
                            AgentsResponseItem::Spawn(spawn_item),
                        )),
                    }));
                }
                Err(e) => yield Err(e),
            }
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::queue::deliver as sdk;
    use objectiveai_sdk::cli::command::agents::queue::deliver::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::queue::deliver as sdk;
    use objectiveai_sdk::cli::command::agents::queue::deliver::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
