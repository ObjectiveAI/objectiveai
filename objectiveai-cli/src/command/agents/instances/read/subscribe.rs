//! `agents read subscribe` — one-shot snapshot read of the queue.
//!
//! The Unix-socket event stream that backed the live `subscribe`
//! semantics is gone. Until the leaf is re-architected on a different
//! transport, the handler returns the current queue contents for the
//! target hierarchy (matching the filter if any) and exits.
//!
//! - `Items { agent_id, items }` — at least one queued item exists.
//! - `Inactive { agent_id }` — no items in the queue and no record
//!   of any prior messages for this hierarchy.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::{
    Request, RequestMessageKind, ResponseItem, SubscribeTarget,
};
use tokio::sync::mpsc;

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::logs::queue::QueueItem;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let kind_filter = request.kind;
    let fs = ctx.filesystem.clone();
    let db = ctx.db.clone();
    let (parent, spawned, sub_id) = match request.target {
        SubscribeTarget::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            let spawned = format!("{parent}/{agent_instance}");
            (parent, spawned, agent_instance)
        }
        SubscribeTarget::Tag { agent_tag } => resolve_tag(&db, agent_tag).await?,
    };

    let (tx, rx) = mpsc::channel::<Result<ResponseItem, Error>>(16);
    tokio::spawn(async move {
        let result = snapshot(fs, db, parent, spawned, sub_id, kind_filter, &tx).await;
        if let Err(e) = result {
            let _ = tx.send(Err(e)).await;
        }
    });
    Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
}

/// Resolve a `--agent-tag` to the `(parent, spawned, leaf)` triple
/// the rest of the handler expects. BOUND tags split into parent +
/// leaf via [`crate::db::tags::parent_of`] / [`leaf_of`]; PENDING and
/// ABSENT both raise structured errors so the caller sees why the
/// lookup failed.
async fn resolve_tag(
    db: &crate::db::Pool,
    agent_tag: String,
) -> Result<(String, String, String), Error> {
    use crate::db::tags;
    match tags::lookup(db, &agent_tag).await? {
        tags::LookupState::Bound { agent_instance_hierarchy } => {
            let parent = tags::parent_of(&agent_instance_hierarchy).to_string();
            let leaf = tags::leaf_of(&agent_instance_hierarchy).to_string();
            Ok((parent, agent_instance_hierarchy, leaf))
        }
        tags::LookupState::Pending {
            parent_agent_instance_hierarchy,
            agent_full_id,
        } => Err(Error::TagPending {
            tag: agent_tag,
            parent_agent_instance_hierarchy,
            agent_full_id,
        }),
        tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
    }
}

async fn snapshot(
    fs: crate::filesystem::Client,
    db: crate::db::Pool,
    caller: String,
    spawned: String,
    sub_id: String,
    kind_filter: Option<RequestMessageKind>,
    tx: &mpsc::Sender<Result<ResponseItem, Error>>,
) -> Result<(), Error> {
    let items = fs.read_new_from_queue(&db, &caller, &spawned).await?;
    let filtered: Vec<QueueItem> = items
        .into_iter()
        .filter(|item| match kind_filter {
            None => true,
            Some(k) => queue_item_kind(item) == k,
        })
        .collect();
    if filtered.is_empty() {
        let _ = tx
            .send(Ok(ResponseItem::Inactive { agent_id: sub_id }))
            .await;
    } else {
        let _ = tx
            .send(Ok(ResponseItem::Items {
                agent_id: sub_id,
                items: filtered,
            }))
            .await;
    }
    Ok(())
}

fn queue_item_kind(item: &QueueItem) -> RequestMessageKind {
    match item {
        QueueItem::AssistantResponse { .. } => RequestMessageKind::AssistantResponse,
        QueueItem::ToolResponse { .. } => RequestMessageKind::ToolResponse,
        QueueItem::Notification { .. } => RequestMessageKind::AgentCompletionNotification,
        QueueItem::AgentCompletionRequest { .. } => RequestMessageKind::AgentCompletionRequest,
        QueueItem::FunctionExecutionRequest { .. } => RequestMessageKind::FunctionExecutionRequest,
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
