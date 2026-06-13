//! `agents queue deliver` — wake every queue-pending target in the
//! caller's subtree.
//!
//! Targets come from `db::message_queue::list_delivery_targets`, in
//! two kinds: AIHs with active queued prompts (direct rows + rows
//! parked against BOUND tags, resolved; deduped, caller itself
//! excluded), and un-upgraded (GROUPED) tags whose group parent sits
//! in the subtree. Per target, the matching lock
//! ([`crate::command::agents::locks`]) is try-acquired with no
//! waiting:
//!
//! - lock held by a live owner → `AgentActive {aih}` / `TagActive
//!   {tag}` — the agent is already running (or the tag is already
//!   being materialized) and will drain its own queue;
//! - lock won → `AgentSpawned {aih}` / `TagSpawned {tag}`, then the
//!   SAME spawn machinery `agents spawn` / `agents message` use
//!   (`spawn::run_multi_pass`, empty messages; AIHs resume via the
//!   stored continuation, tags spawn fresh from the group's stored
//!   agent spec with the tag threaded into the conduit upgrade)
//!   streams the agent's output as `Value {aih, value}` envelopes.
//!   An AIH claim is preseeded into the run's
//!   [`AgentInstanceRegistry`], so the lock is released the moment
//!   THAT task's stream ends — never held for the slowest. A tag
//!   claim goes in via `hold_tag_claim`: released as soon as the
//!   spawn claims its minted AIH lock (first chunk, just before the
//!   `Id` first item), held to stream end otherwise. For tag spawns
//!   the minted AIH isn't known up front — it arrives as the FIRST
//!   inner item (the spawn `Id`), which also keys the `Value`
//!   envelopes.
//!
//! Each per-target stream's FIRST item is always its resolution
//! (`AgentActive` / `AgentSpawned` / `TagActive` / `TagSpawned` / a
//! setup `Err`); once every target has resolved, the bare string
//! `"AllAgentsActive"` is emitted mid-stream and spawn output keeps
//! flowing after it.
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
use objectiveai_sdk::cli::command::ResponseItem as RootResponseItem;
use objectiveai_sdk::cli::command::agents::ResponseItem as AgentsResponseItem;
use objectiveai_sdk::cli::command::agents::queue::deliver::{
    AgentActiveResponseItem, AgentActiveType, AgentSpawnedResponseItem, AgentSpawnedType,
    AllAgentsActive, Request, RequestDangerousAdvanced, ResponseItem, TagActiveResponseItem,
    TagActiveType, TagSpawnedResponseItem, TagSpawnedType, ValueResponseItem,
};
use objectiveai_sdk::cli::command::agents::spawn::ResponseItem as SpawnResponseItem;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::db;
use crate::error::Error;
use crate::websockets::agent_registry::AgentInstanceRegistry;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

/// Internal merge item: each per-target stream is tagged with its
/// index so the outer driver can tell when every target has resolved
/// (each stream's first item is its resolution by construction).
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
/// child's STATUS items (`AgentActive` / `AgentSpawned` /
/// `TagActive` / `TagSpawned` — `Value` spawn output is skipped) up
/// to and including `AllAgentsActive`, and return.
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
    // Re-exec of this CLI — strip the parent-only envelope fields.
    crate::command::reexec::strip_inherited(&mut child_request.base);

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
    // Queue-pending targets in the caller's subtree: AIHs (caller
    // excluded — deliver targets only strict descendants; the query
    // is parent-inclusive) and un-upgraded tags.
    let caller = ctx.config.agent_instance_hierarchy.clone();
    let targets = db::message_queue::list_delivery_targets(ctx.db_client().await?, &caller).await?;
    let mut hierarchies: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    for target in targets {
        match target {
            db::message_queue::DeliveryTarget::Hierarchy { agent_instance_hierarchy } => {
                if agent_instance_hierarchy != caller
                    && !hierarchies.contains(&agent_instance_hierarchy)
                {
                    hierarchies.push(agent_instance_hierarchy);
                }
            }
            db::message_queue::DeliveryTarget::GroupedTag { agent_tag } => {
                if !tags.contains(&agent_tag) {
                    tags.push(agent_tag);
                }
            }
        }
    }

    let n = hierarchies.len() + tags.len();
    let mut select_all = futures::stream::SelectAll::new();
    let mut idx = 0usize;
    for hierarchy in hierarchies {
        let i = idx;
        idx += 1;
        let tagged = deliver_one_hierarchy(ctx.clone(), hierarchy).map(move |item| (i, item));
        select_all.push(Box::pin(tagged) as TaggedStream);
    }
    for tag in tags {
        let i = idx;
        idx += 1;
        let tagged = deliver_one_tag(ctx.clone(), tag).map(move |item| (i, item));
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
            // Every per-target stream's first item is its resolution —
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
/// session). On a win, the claim is preseeded into the run's
/// [`AgentInstanceRegistry`] — released when that stream (and the
/// registry inside it) drops, i.e. per-target, never held for the
/// slowest.
fn deliver_one_hierarchy(
    ctx: Context,
    hierarchy: String,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::stream! {
        let state_dir = ctx.filesystem.state_dir();
        let (dir, key) =
            crate::command::agents::locks::agent_instance_lock(&state_dir, &hierarchy);
        let Some(claim) = objectiveai_sdk::lockfile::try_acquire(&dir, &key, "").await else {
            yield Ok(ResponseItem::AgentActive(AgentActiveResponseItem {
                r#type: AgentActiveType::AgentActive,
                agent_instance_hierarchy: hierarchy,
            }));
            return;
        };

        let pool = match ctx.db_client().await {
            Ok(pool) => pool,
            Err(e) => {
                let _ = claim.release();
                yield Err(e);
                return;
            }
        };
        let lookup = match crate::db::logs::lookup_session(pool, &hierarchy).await {
            Ok(Some(lookup)) => lookup,
            Ok(None) => {
                // SDK claims don't release on drop — free the slot
                // before bailing.
                let _ = claim.release();
                yield Err(Error::AgentNoPriorRequest {
                    agent_instance_hierarchy: hierarchy,
                });
                return;
            }
            Err(e) => {
                let _ = claim.release();
                yield Err(e.into());
                return;
            }
        };

        let mut registry = AgentInstanceRegistry::new(state_dir);
        registry.preseed(hierarchy.clone(), claim);

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
            registry,
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

/// Deliver one un-upgraded (GROUPED) tag. The FIRST item is always
/// the resolution: `TagActive` (tag lock held — someone else is
/// already materializing it), `TagSpawned` (tag lock won, fresh
/// spawn of the group's stored spec starting), or — when the tag
/// raced to BOUND between the target listing and the lock — the
/// delegated hierarchy flow's own resolution. The tag claim rides in
/// the run's registry via `hold_tag_claim`: released the moment the
/// spawn claims its minted AIH lock, held to stream end otherwise.
/// The minted AIH arrives as the FIRST inner item (the spawn `Id`)
/// and keys the `Value` envelopes.
fn deliver_one_tag(
    ctx: Context,
    agent_tag: String,
) -> impl Stream<Item = Result<ResponseItem, Error>> + Send {
    async_stream::stream! {
        let state_dir = ctx.filesystem.state_dir();
        let (dir, key) =
            crate::command::agents::locks::agent_tag_lock(&state_dir, &agent_tag);
        let Some(claim) = objectiveai_sdk::lockfile::try_acquire(&dir, &key, "").await else {
            yield Ok(ResponseItem::TagActive(TagActiveResponseItem {
                r#type: TagActiveType::TagActive,
                agent_tag,
            }));
            return;
        };

        // Re-resolve under the lock — the target list was a snapshot
        // and the tag may have been upgraded (or deleted) since.
        let pool = match ctx.db_client().await {
            Ok(pool) => pool,
            Err(e) => {
                let _ = claim.release();
                yield Err(e);
                return;
            }
        };
        let agent = match crate::db::tags::lookup(pool, &agent_tag).await {
            Ok(crate::db::tags::LookupState::Grouped { agent_spec, .. }) => {
                agent_spec
            }
            Ok(crate::db::tags::LookupState::Bound { agent_instance_hierarchy }) => {
                // Raced to BOUND — the tag lock has no further job;
                // deliver the live hierarchy instead.
                let _ = claim.release();
                let mut inner = Box::pin(deliver_one_hierarchy(
                    ctx.clone(),
                    agent_instance_hierarchy,
                ));
                while let Some(item) = inner.next().await {
                    yield item;
                }
                return;
            }
            Ok(crate::db::tags::LookupState::Absent) => {
                let _ = claim.release();
                yield Err(Error::TagNotFound(agent_tag));
                return;
            }
            Err(e) => {
                let _ = claim.release();
                yield Err(e.into());
                return;
            }
        };

        let mut registry = AgentInstanceRegistry::new(state_dir);
        registry.hold_tag_claim(claim);

        yield Ok(ResponseItem::TagSpawned(TagSpawnedResponseItem {
            r#type: TagSpawnedType::TagSpawned,
            agent_tag: agent_tag.clone(),
        }));

        // Fresh spawn from the group's stored spec: empty messages
        // (the queued rows ARE the prompt, drained via the conduit),
        // no continuation, the tag threaded in so the first conduit
        // read flips the whole group to BOUND on the minted AIH.
        let params = AgentCompletionCreateParams {
            messages: Vec::new(),
            provider: None,
            agent,
            response_format: None,
            seed: None,
            stream: Some(true),
            continuation: None,
        };
        let inner = crate::command::agents::spawn::run_multi_pass(
            ctx.clone(),
            params,
            Some(agent_tag),
            registry,
        );
        let mut inner = Box::pin(inner);
        // The minted AIH keys the Value envelopes; the spawn stream's
        // first item is always its Id, so it's in hand before any
        // chunk needs wrapping.
        let mut minted: Option<String> = None;
        while let Some(item) = inner.next().await {
            match item {
                Ok(spawn_item) => {
                    if let SpawnResponseItem::Id(id) = &spawn_item {
                        minted = Some(id.clone());
                    }
                    let aih = minted.clone().unwrap_or_default();
                    yield Ok(ResponseItem::Value(ValueResponseItem {
                        agent_instance_hierarchy: aih,
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
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
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
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
