//! `agents logs read subscribe` — first-ping-or-go-inactive
//! wait loop.
//!
//! Outer loop:
//! 1. Existence check (side-effect-free) per target, filtered by
//!    the optional kinds. If ANY target has a matching unread row
//!    past its watermark, call `read_pending_for_parent` for ALL
//!    targets (UNFILTERED — kinds gate "wake up", not the
//!    returned slice) and emit every block. Close.
//! 2. Else: lock check. If no target's file lock is held, emit
//!    `AgentsInactive` and close.
//! 3. Else: per held-target, race `wait_for_logs_message_at` vs
//!    `lock_file::wait_release`. First fire wins. DB ping →
//!    restart from step 1 (but loop back to existence check —
//!    type-filter false positives are dropped silently). Lock
//!    release → restart from step 1.

use std::path::PathBuf;
use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use objectiveai_sdk::cli::command::agents::logs::read::subscribe::{
    AgentsInactiveTag, KindFilter, Request, ResponseItem, Target,
};

use crate::context::Context;
use crate::db::logs::MessageTable;
use crate::db::tags;
use crate::error::Error;
use crate::lock_file;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

#[derive(Clone)]
struct Resolved {
    spawned: String,
    lock_path: PathBuf,
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let db = ctx.db.get().await?.clone();
    let agents_dir = ctx
        .filesystem
        .state_dir()
        .join("instances")
        .join("agents");
    // Match the dir-create posture used by `agents message`.
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| Error::Instance(format!("create agents_dir: {e}")))?;

    // Resolve every target up front. Tags resolve via tags::lookup;
    // PENDING/GROUPED/ABSENT raises a structured error before we
    // commit to the wait loop.
    let mut resolved: Vec<Resolved> = Vec::with_capacity(request.targets.len());
    for target in request.targets {
        let spawned = resolve_target(&db, target, &default_parent).await?;
        let lock_path = agents_dir.join(spawned.replace('/', "_"));
        resolved.push(Resolved { spawned, lock_path });
    }

    let kinds = build_kind_filter(request.kinds);
    let after_id = request.after_id;
    let limit = request.limit;

    let stream = async_stream::stream! {
        loop {
            // Step 1: existence check per target, filtered by kinds.
            let mut anything_matches = false;
            for r in &resolved {
                let matched = match crate::db::logs::any_pending_matching_kinds(
                    &db, &r.spawned, after_id, kinds.as_deref(),
                ).await {
                    Ok(v) => v,
                    Err(e) => { yield Err(Error::from(e)); return; }
                };
                if matched {
                    anything_matches = true;
                    break;
                }
            }

            if anything_matches {
                // A matching row exists somewhere. Drain UNFILTERED
                // pending for every target (kinds gate "wake up",
                // not the returned slice). Emit and close.
                for r in &resolved {
                    let items = match crate::db::logs::read_pending_for_parent(
                        &db, &r.spawned, after_id, limit,
                    ).await {
                        Ok(v) => v,
                        Err(e) => { yield Err(Error::from(e)); return; }
                    };
                    for inner in items {
                        yield Ok(ResponseItem::Item(inner));
                    }
                }
                return;
            }

            // Step 2: filter to held-lock targets.
            let held: Vec<Resolved> = resolved
                .iter()
                .filter(|r| lock_file::is_held(&r.lock_path))
                .cloned()
                .collect();
            if held.is_empty() {
                yield Ok(ResponseItem::AgentsInactive(
                    AgentsInactiveTag::AgentsInactive,
                ));
                return;
            }

            // Step 3: per-target subscription race. First to fire wins;
            // restart the loop regardless of which arm fired.
            // - DB ping: a new logs.messages row landed at that AIH;
            //   the existence check at the top will re-decide if it
            //   matched our kinds filter.
            // - Lock release: that target dropped; the next iteration
            //   re-evaluates the held set.
            let mut watch_set = FuturesUnordered::new();
            for r in &held {
                let db = db.clone();
                let spawned = r.spawned.clone();
                let lock_path = r.lock_path.clone();
                watch_set.push(async move { watch_target(&db, spawned, lock_path).await });
            }
            match watch_set.next().await {
                Some(Ok(())) => continue,
                Some(Err(e)) => { yield Err(e); return; }
                None => unreachable!("held set is non-empty"),
            }
        }
    };
    Ok(Box::pin(stream))
}

/// Race the postgres LISTEN against the filesystem lock release.
/// Returns `Ok(())` on either fire — the outer loop doesn't care
/// which one woke us up; it re-runs the existence check + lock
/// check from the top.
async fn watch_target(
    pool: &crate::db::Pool,
    target_aih: String,
    lock_path: PathBuf,
) -> Result<(), Error> {
    tokio::select! {
        result = crate::db::logs::wait_for_logs_message_at(pool, &target_aih) => {
            result.map_err(Error::from)
        }
        result = lock_file::wait_release(&lock_path) => {
            result.map_err(|e| Error::Instance(format!(
                "lock release wait: {e}"
            )))
        }
    }
}

/// Direct mode → `{parent}/{instance}`. Tag mode resolves via
/// `tags::lookup`; PENDING / ABSENT raise structured errors.
async fn resolve_target(
    db: &crate::db::Pool,
    target: Target,
    default_parent: &str,
) -> Result<String, Error> {
    match target {
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent =
                parent_agent_instance_hierarchy.unwrap_or_else(|| default_parent.to_string());
            Ok(format!("{parent}/{agent_instance}"))
        }
        // The caller's own AIH itself — no child leaf appended.
        Target::Me => Ok(default_parent.to_string()),
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound { agent_instance_hierarchy } => {
                Ok(agent_instance_hierarchy)
            }
            tags::LookupState::Grouped {
                tag_group_id,
                parent_agent_instance_hierarchy,
                ..
            } => Err(Error::TagGrouped {
                tag: agent_tag,
                tag_group_id,
                parent_agent_instance_hierarchy,
            }),
            tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        },
    }
}

/// Translate the 3-bool `KindFilter` into the `MessageTable`
/// union the DB existence check accepts. Returns `None` when no
/// flag is set (no filter — wake on any kind).
///
/// Mapping:
/// - `request` → 3 `*_request` variants + 5 `message_queue_*`
///   variants (8 total — request blobs and client notifications
///   share this bucket per the spec).
/// - `assistant` → 8 `assistant_response_*` variants.
/// - `tool` → 6 `tool_response*` variants.
fn build_kind_filter(filter: Option<KindFilter>) -> Option<Vec<MessageTable>> {
    let f = filter?;
    let mut kinds: Vec<MessageTable> = Vec::new();
    if f.request {
        kinds.extend_from_slice(&[
            MessageTable::AgentCompletionRequest,
            MessageTable::VectorCompletionRequest,
            MessageTable::FunctionExecutionRequest,
            MessageTable::MessageQueueText,
            MessageTable::MessageQueueImage,
            MessageTable::MessageQueueAudio,
            MessageTable::MessageQueueVideo,
            MessageTable::MessageQueueFile,
        ]);
    }
    if f.assistant {
        kinds.extend_from_slice(&[
            MessageTable::AssistantResponseRefusal,
            MessageTable::AssistantResponseReasoning,
            MessageTable::AssistantResponseToolCalls,
            MessageTable::AssistantResponseContentText,
            MessageTable::AssistantResponseContentImage,
            MessageTable::AssistantResponseContentAudio,
            MessageTable::AssistantResponseContentVideo,
            MessageTable::AssistantResponseContentFile,
        ]);
    }
    if f.tool {
        kinds.extend_from_slice(&[
            MessageTable::ToolResponse,
            MessageTable::ToolResponseContentText,
            MessageTable::ToolResponseContentImage,
            MessageTable::ToolResponseContentAudio,
            MessageTable::ToolResponseContentVideo,
            MessageTable::ToolResponseContentFile,
        ]);
    }
    if kinds.is_empty() { None } else { Some(kinds) }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::logs::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::logs::read::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::logs::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::logs::read::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
