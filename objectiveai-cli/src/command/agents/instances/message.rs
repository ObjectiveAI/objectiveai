//! `agents instances message` — stream-aware delivery primitive.
//!
//! See the SDK's `agents::instances::message` module doc for the
//! external semantics. The cli-side decision tree:
//!
//! 1. **Resolve target.** Direct → compose `{parent}/{instance}`.
//!    Tag → look up in `tags`; BOUND falls through as a Direct
//!    equivalent, PENDING/ABSENT falls back to pure enqueue
//!    against the tag (or errors when message is `None`).
//! 2. **Resolve content.** `Some(rm)` → `resolve_message` →
//!    `RichContent`. `None` → tracked through the rest of the
//!    flow; the eventual spawn (if any) gets an empty `messages`
//!    vec.
//! 3. **Branch on `dangerous_advanced.stream`:**
//!    - `stream=true` → `execute_streaming`.
//!    - `stream=false | None` → `execute_unary`.

use std::path::PathBuf;
use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::agent::completions::message::{Message, RichContent, UserMessage};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Request, RequestDangerousAdvanced, RequestMessage, ResponseItem,
};
use objectiveai_sdk::cli::command::agents::instances::spawn::ResponseItem as SpawnResponseItem;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::db::tags::LookupState;
use crate::error::Error;
use crate::websockets::lock_file;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let want_stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    let agents_dir = ctx
        .filesystem
        .base_dir()
        .join("instances")
        .join("agents");

    // Phase 1: resolve target → (hierarchy, tag) or early-return.
    let (hierarchy, _tag) = match resolve_target(ctx, &request).await? {
        ResolvedTarget::Hierarchy { hierarchy, tag } => (hierarchy, tag),
        ResolvedTarget::EnqueueAgainstTag { id, agent_tag } => {
            return Ok(once_item(ResponseItem::Enqueued {
                id,
                agent_instance_hierarchy: None,
                agent_tag: Some(agent_tag),
            }));
        }
    };

    // Phase 2: resolve the user payload — always required.
    let message_content = resolve_message(request.message.clone())?;

    if want_stream {
        execute_streaming(ctx, hierarchy, message_content, agents_dir).await
    } else {
        execute_unary(ctx, hierarchy, message_content, agents_dir, request).await
    }
}

enum ResolvedTarget {
    /// Successfully resolved to a concrete hierarchy (Direct or
    /// BOUND tag). Caller continues into the lock-race flow.
    Hierarchy {
        hierarchy: String,
        #[allow(dead_code)] // exposed for future tag-binding hooks.
        tag: Option<String>,
    },
    /// Tag was unresolvable (PENDING / ABSENT). The message was
    /// already enqueued against the tag name; caller short-circuits
    /// with the assigned queue row id.
    EnqueueAgainstTag { id: i64, agent_tag: String },
}

async fn resolve_target(ctx: &Context, request: &Request) -> Result<ResolvedTarget, Error> {
    match &request.target {
        MessageTarget::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            Ok(ResolvedTarget::Hierarchy {
                hierarchy: format!("{parent}/{agent_instance}"),
                tag: None,
            })
        }
        MessageTarget::Tag { agent_tag } => {
            let state = crate::db::tags::lookup(&ctx.db, agent_tag).await?;
            match state {
                LookupState::Bound {
                    agent_instance_hierarchy,
                } => Ok(ResolvedTarget::Hierarchy {
                    hierarchy: agent_instance_hierarchy,
                    tag: Some(agent_tag.clone()),
                }),
                LookupState::Grouped { .. } | LookupState::Absent => {
                    // No live target. Pure enqueue against the
                    // tag name — the queue reader resolves it later.
                    let content = resolve_message(request.message.clone())?;
                    let id = crate::db::message_queue::enqueue_with_content(
                        &ctx.db,
                        None,
                        Some(agent_tag.clone()),
                        None,
                        content,
                    )
                    .await?;
                    Ok(ResolvedTarget::EnqueueAgainstTag {
                        id,
                        agent_tag: agent_tag.clone(),
                    })
                }
            }
        }
    }
}

/// `stream=true`: try to acquire the lock immediately. If we get
/// it, skip enqueue + run spawn in-process. If we don't, enqueue +
/// race DB-delivery against blocking lock acquisition.
async fn execute_streaming(
    ctx: &Context,
    hierarchy: String,
    message_content: RichContent,
    agents_dir: PathBuf,
) -> Result<ItemStream, Error> {
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| Error::Instance(format!("create agents_dir: {e}")))?;
    let lock_path = agents_dir.join(hierarchy.replace('/', "_"));

    // Fast path: nobody holds the lock — we win it now.
    if let Some(claim) = lock_file::try_acquire(&lock_path) {
        return Ok(run_spawn_with(ctx, claim, hierarchy, message_content).await?);
    }

    // Slow path: live owner. Enqueue + race.
    let queue_id = crate::db::message_queue::enqueue_with_content(
        &ctx.db,
        Some(hierarchy.clone()),
        None,
        None,
        message_content.clone(),
    )
    .await?;

    let pool = ctx.db.clone();
    let lock_path_clone = lock_path.clone();
    tokio::select! {
        delivery = crate::db::message_queue::subscribe_delivered(&pool, queue_id) => {
            delivery?;
            Ok(once_item(ResponseItem::Delivered))
        }
        claim = lock_file::wait_acquire(&lock_path_clone) => {
            let claim = claim.map_err(|e| Error::Instance(format!(
                "lock acquisition: {e}"
            )))?;
            // We're the new owner. Reclaim our queue row before
            // spawning — the conduit shouldn't see it again.
            let _ = crate::db::message_queue::delete_by_id(&ctx.db, queue_id).await;
            run_spawn_with(ctx, claim, hierarchy, message_content).await
        }
    }
}

/// Drive `spawn::run_multi_pass` for this hierarchy. Pulls the
/// agent definition + latest continuation out of the logs DB
/// (single round-trip), pre-builds an
/// `AgentCompletionCreateParams` with the continuation stamped,
/// and threads the held `LockClaim` through the returned stream's
/// state so the lock is owned for the full spawn lifetime.
async fn run_spawn_with(
    ctx: &Context,
    claim: lock_file::LockClaim,
    hierarchy: String,
    message_content: RichContent,
) -> Result<ItemStream, Error> {
    let lookup = crate::db::logs::lookup_session(&ctx.db, &hierarchy)
        .await?
        .ok_or_else(|| {
            Error::Instance(format!("no prior session for {hierarchy:?}"))
        })?;

    let messages = vec![Message::User(UserMessage {
        content: message_content,
        name: None,
    })];
    let params = AgentCompletionCreateParams {
        messages,
        provider: None,
        agent: lookup.agent,
        response_format: None,
        seed: None,
        stream: Some(true),
        continuation: lookup.continuation,
    };
    let agents_dir = ctx
        .filesystem
        .base_dir()
        .join("instances")
        .join("agents");

    let inner = crate::command::agents::instances::spawn::run_multi_pass(
        ctx.clone(),
        params,
        None,
        agents_dir,
    );

    // Tie `claim` into the stream's lifetime — drop = release
    // happens only when the consumer drops our output stream.
    let stream = async_stream::try_stream! {
        let _claim = claim;
        let mut inner = Box::pin(inner);
        while let Some(item) = inner.next().await {
            let ev = item?;
            match ev {
                SpawnResponseItem::Id(id) => {
                    yield ResponseItem::Id {
                        agent_instance_hierarchy: id,
                    }
                }
                SpawnResponseItem::Chunk(c) => yield ResponseItem::Chunk(c),
            }
        }
    };
    Ok(Box::pin(stream))
}

/// `stream=false`: check the lock. If held → enqueue + race
/// DB-delivery against lock-release; on release, delete the queue
/// row and re-check. If not held → re-exec ourselves as a detached
/// subprocess with `stream=true`, take the child's first item.
async fn execute_unary(
    ctx: &Context,
    hierarchy: String,
    message_content: RichContent,
    agents_dir: PathBuf,
    request: Request,
) -> Result<ItemStream, Error> {
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| Error::Instance(format!("create agents_dir: {e}")))?;
    let lock_path = agents_dir.join(hierarchy.replace('/', "_"));

    loop {
        if lock_file::is_held(&lock_path) {
            // Live agent — enqueue + race delivery vs release.
            let queue_id = crate::db::message_queue::enqueue_with_content(
                &ctx.db,
                Some(hierarchy.clone()),
                None,
                None,
                message_content.clone(),
            )
            .await?;

            let pool = ctx.db.clone();
            let lock_path_clone = lock_path.clone();
            let race_outcome = tokio::select! {
                delivery = crate::db::message_queue::subscribe_delivered(&pool, queue_id) => {
                    delivery?;
                    RaceOutcome::Delivered
                }
                release = lock_file::wait_release(&lock_path_clone) => {
                    release.map_err(|e| Error::Instance(format!(
                        "lock release wait: {e}"
                    )))?;
                    RaceOutcome::Released
                }
            };
            match race_outcome {
                RaceOutcome::Delivered => {
                    return Ok(once_item(ResponseItem::Delivered));
                }
                RaceOutcome::Released => {
                    // Lock dropped — delete the queue row and
                    // loop back to re-check (a new live agent may
                    // have claimed it already, or no one will and
                    // we'll fall through to the detached respawn).
                    let _ = crate::db::message_queue::delete_by_id(&ctx.db, queue_id).await;
                    continue;
                }
            }
        }

        // No live agent — re-exec the cli as a detached subprocess
        // running this same command with stream=true. The child
        // will acquire the lock and drive `spawn::run_multi_pass`.
        return execute_unary_respawn(&hierarchy, &request).await;
    }
}

enum RaceOutcome {
    Delivered,
    Released,
}

async fn execute_unary_respawn(
    hierarchy: &str,
    original: &Request,
) -> Result<ItemStream, Error> {
    // Convert Tag → Direct (the subprocess sees the resolved
    // hierarchy, not the original tag).
    let (parent, leaf) = match hierarchy.rsplit_once('/') {
        Some((p, l)) => (p.to_string(), l.to_string()),
        None => (String::new(), hierarchy.to_string()),
    };
    let mut child_request = original.clone();
    child_request.target = MessageTarget::Direct {
        parent_agent_instance_hierarchy: Some(parent),
        agent_instance: leaf,
    };
    child_request.dangerous_advanced = Some(RequestDangerousAdvanced {
        stream: Some(true),
    });

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let executor = BinaryExecutor::from_path(exe).detach(true);
    let mut stream = executor
        .execute::<Request, ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!(
            "self-respawn for agents instances message: {e}"
        )))?;
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!(
            "self-respawn for agents instances message: {e}"
        )))?;
    Ok(once_item(first))
}

fn once_item(item: ResponseItem) -> ItemStream {
    Box::pin(futures::stream::once(async move {
        Ok::<ResponseItem, Error>(item)
    }))
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
    use objectiveai_sdk::cli::command::agents::instances::message as sdk;
    use objectiveai_sdk::cli::command::agents::instances::message::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::message as sdk;
    use objectiveai_sdk::cli::command::agents::instances::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
