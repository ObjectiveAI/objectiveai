//! `agents message` — unary delivery primitive.
//!
//! The agent input is the shared [`AgentSelector`] — the same shape
//! `agents spawn` takes, so an unspawned agent can be messaged:
//!
//! - **ref**: nothing to lock — exec a detached `agents spawn`
//!   child (stream=true) carrying the resolved agent + message
//!   inline, return its first item as `Id`.
//! - **instance / BOUND tag** (the AIH lock) and **GROUPED tag**
//!   (the tag lock): try_acquire. Won → exec the spawn child with
//!   `skip_lock=true`; an AIH claim is TRANSFERRED into the child
//!   (the lock then lives exactly as long as the child), a TAG claim
//!   is held by THIS process instead — only AIH locks ever transfer
//!   — and persists until this process exits → `Id`. Lost → enqueue
//!   the message, then race `subscribe_delivered` (the row flipping
//!   inactive means a live agent consumed it → `Delivered`) against
//!   `wait_acquire` (the slot freed up first → reclaim our queue
//!   row, exec the spawn child with the fresh claim → `Id`).
//!
//! The message payload is resolved ONCE in this process (file IO /
//! Python run here); spawn children always receive the resolved
//! `RichContent` via `--inline`. Fire-and-forget parking without
//! the race lives in `agents enqueue`.

use futures::StreamExt;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::message::{Request, RequestMessage, Response};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn as spawn_sdk;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};
use objectiveai_sdk::lockfile::LockClaim;

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request {
        agent,
        message,
        dangerous_advanced,
        ..
    } = request;
    let seed = dangerous_advanced.as_ref().and_then(|a| a.seed);

    // Resolve the payload once, in this process.
    let content = resolve_message(message)?;

    let state_dir = ctx.filesystem.state_dir();
    let route = match agent {
        AgentSelector::Ref { agent } => {
            // Resolve file/python refs HERE too — the child gets the
            // typed agent inline and never re-runs the Python.
            let resolved = super::spawn::resolve_agent_ref(agent)?;
            Route::Ref {
                child: AgentSelector::Ref {
                    agent: AgentRef::Resolved(resolved),
                },
            }
        }
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            instance_route(&state_dir, format!("{parent}/{agent_instance}"))
        }
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                } => instance_route(&state_dir, agent_instance_hierarchy),
                crate::db::tags::LookupState::Grouped { .. } => {
                    let (dir, key) = super::locks::agent_tag_lock(&state_dir, &agent_tag);
                    Route::Locked {
                        dir,
                        key,
                        hierarchy: None,
                        tag: Some(agent_tag.clone()),
                        child: AgentSelector::Tag { agent_tag },
                    }
                }
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
    };

    match route {
        Route::Ref { child } => spawn_child(child, content, seed, None, false).await,
        Route::Locked {
            dir,
            key,
            hierarchy,
            tag,
            child,
        } => {
            let is_tag = tag.is_some();
            // Fast path: nobody holds the lock — exec the spawn child
            // under the fresh claim.
            if let Some(claim) = objectiveai_sdk::lockfile::try_acquire(&dir, &key, "").await {
                return spawn_locked(child, content, seed, claim, is_tag).await;
            }

            // Slow path: live owner. Park the message, then wait for
            // whichever comes first — the row flipping inactive (the
            // owner consumed it) or the lock freeing up (the owner
            // died / finished without consuming; we take over).
            let queue_id = crate::db::message_queue::enqueue_with_content(
                ctx.db_client().await?,
                hierarchy,
                tag,
                &ctx.config.agent_instance_hierarchy,
                None,
                content.clone(),
            )
            .await?;
            let pool = ctx.db_client().await?.clone();
            tokio::select! {
                delivery = crate::db::message_queue::subscribe_delivered(&pool, queue_id) => {
                    delivery?;
                    Ok(Response::Delivered)
                }
                claim = objectiveai_sdk::lockfile::wait_acquire(&dir, &key, "") => {
                    let claim = claim.map_err(|e| Error::Lockfile {
                        key: key.clone(),
                        source: e,
                    })?;
                    // We own the slot now. Reclaim our queue row so
                    // the spawn child doesn't see it again — the
                    // message rides inline instead.
                    let _ = crate::db::message_queue::delete_by_id(
                        ctx.db_client().await?,
                        queue_id,
                        &ctx.config.agent_instance_hierarchy,
                    )
                    .await;
                    spawn_locked(child, content, seed, claim, is_tag).await
                }
            }
        }
    }
}

/// Resolved delivery route for the non-enqueue flow.
enum Route {
    /// Plain agent ref — nothing to lock or enqueue against; always
    /// spawns a fresh agent carrying the message.
    Ref { child: AgentSelector },
    /// Lockable target: the lock coordinates, the queue target
    /// (exactly one of `hierarchy` / `tag` is `Some`), and the
    /// selector the spawn child receives.
    Locked {
        dir: std::path::PathBuf,
        key: String,
        hierarchy: Option<String>,
        tag: Option<String>,
        child: AgentSelector,
    },
}

/// Route for a fully resolved `agent_instance_hierarchy`: the AIH
/// lock, queued against the hierarchy, child re-addressed as an
/// explicit Instance (parent + leaf) so it doesn't depend on the
/// child process's own identity.
fn instance_route(state_dir: &std::path::Path, hierarchy: String) -> Route {
    let (dir, key) = super::locks::agent_instance_lock(state_dir, &hierarchy);
    let child = AgentSelector::Instance {
        parent_agent_instance_hierarchy: Some(
            crate::db::tags::parent_of(&hierarchy).to_string(),
        ),
        agent_instance: crate::db::tags::leaf_of(&hierarchy).to_string(),
    };
    Route::Locked {
        dir,
        key,
        hierarchy: Some(hierarchy),
        tag: None,
        child,
    }
}

/// Exec the spawn child for a freshly won lock. AIH claims TRANSFER
/// into the child (the lock then lives exactly as long as the
/// child); TAG claims never transfer — this process keeps holding
/// the claim while the child runs with `skip_lock`, and since
/// dropping a claim does not release it, the tag lock persists until
/// this process exits. That's deliberate: nothing else may
/// materialize the tag while the child is still pre-first-chunk; by
/// the time the child's first item (`Id`) arrives it already holds
/// its minted AIH lock, and the GROUPED→BOUND upgrade has landed.
async fn spawn_locked(
    agent: AgentSelector,
    content: RichContent,
    seed: Option<i64>,
    claim: LockClaim,
    is_tag: bool,
) -> Result<Response, Error> {
    if is_tag {
        let _tag_claim = claim;
        spawn_child(agent, content, seed, None, true).await
    } else {
        spawn_child(agent, content, seed, Some(claim), true).await
    }
}

/// Exec a detached `agents spawn` child (stream=true) and return its
/// first item as the unary response. When `transfer` is `Some`, the
/// lock is transferred into the child — the child becomes the sole
/// owner and the lock lives until it exits. `skip_lock` tells the
/// child to skip its own initial acquisition (set whenever a lock is
/// held for it, transferred or not). The child's first item is
/// always its `Id` (chunks are gated behind it); the rest of the
/// stream is dropped and the orphan keeps running.
async fn spawn_child(
    agent: AgentSelector,
    content: RichContent,
    seed: Option<i64>,
    transfer: Option<LockClaim>,
    skip_lock: bool,
) -> Result<Response, Error> {
    let skip_lock = skip_lock.then_some(true);
    let child_request = spawn_sdk::Request {
        path_type: spawn_sdk::Path::AgentsSpawn,
        message: RequestMessage::Inline(content),
        agent,
        dangerous_advanced: Some(spawn_sdk::RequestDangerousAdvanced {
            stream: Some(true),
            seed,
            skip_lock,
        }),
        jq: None,
    };

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let mut executor = BinaryExecutor::from_path(exe).detach(true);
    if let Some(claim) = transfer {
        executor = executor.transfer_lock(claim);
    }
    let mut stream = executor
        .execute::<spawn_sdk::Request, spawn_sdk::ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    match first {
        spawn_sdk::ResponseItem::Id(agent_instance_hierarchy) => Ok(Response::Id {
            agent_instance_hierarchy,
        }),
        spawn_sdk::ResponseItem::Chunk(_) => Err(Error::Instance(
            "agents spawn child emitted a chunk before its id".to_string(),
        )),
    }
}

pub fn resolve_message(message: RequestMessage) -> Result<RichContent, Error> {
    let (simple, inline, file, python_inline, python_file) = match message {
        RequestMessage::Inline(rich) => return Ok(rich),
        RequestMessage::Simple(s) => (Some(s), None, None, None, None),
        RequestMessage::File(p) => (None, None, Some(p), None, None),
        RequestMessage::PythonInline(code) => (None, None, None, Some(code), None),
        RequestMessage::PythonFile(p) => (None, None, None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        simple,
        inline,
        file,
        python_inline,
        python_file,
        RichContent::Text,
    )
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
