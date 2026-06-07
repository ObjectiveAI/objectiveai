//! `agents read subscribe` — channel-backed bare-naked port of the
//! legacy `subscribe_recursive` driver. The async driver runs as a
//! detached task; `ResponseItem`s flow to the caller through a
//! tokio mpsc channel wrapped as a stream.
//!
//! See `agents/read/subscribe.rs` (legacy) for the algorithm — this
//! is a verbatim port modulo the notification/handle plumbing being
//! swapped for typed channel sends.

use std::path::{Path, PathBuf};
use std::pin::Pin;

use futures::Stream;
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::{
    Request, RequestMessageKind, ResponseItem, SubscribeTarget,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::logs::SubscribeEvent;
use crate::filesystem::logs::queue::QueueItem;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let kind_filter = request.kind;
    let fs = ctx.filesystem.clone();
    let db = ctx.db.clone();
    // Resolve the target to `(parent, spawned, sub_id)`. Direct mode
    // mirrors the `agents message` parent-fallback pattern; tag mode
    // looks the tag up via the postgres-backed `tags` tier and errors
    // out on PENDING / ABSENT with structured diagnostics.
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
    let pipes_dir = fs.pipes_dir();

    let (tx, rx) = mpsc::channel::<Result<ResponseItem, Error>>(16);
    tokio::spawn(async move {
        let result =
            subscribe_recursive(fs, db, pipes_dir, parent, spawned, sub_id, kind_filter, &tx).await;
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

fn subscribe_recursive(
    fs: crate::filesystem::Client,
    db: crate::db::Pool,
    pipes_dir: PathBuf,
    caller: String,
    spawned: String,
    sub_id: String,
    kind_filter: Option<RequestMessageKind>,
    tx: &mpsc::Sender<Result<ResponseItem, Error>>,
) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + '_>> {
    Box::pin(async move {
        // INVARIANT 1: open the listener BEFORE the first DB query.
        let listener = try_connect_events(&pipes_dir, &spawned).await;

        let items = fs.read_new_from_queue(&db, &caller, &spawned).await?;
        let mut matched = matches_filter(&items, kind_filter);
        if !items.is_empty() {
            send_items(tx, &sub_id, items).await?;
        }
        if matched {
            return Ok(());
        }

        let mut listener = match listener {
            Some(l) => l,
            None => {
                // INVARIANT 2: second connect attempt before declaring inactive.
                if (try_connect_events(&pipes_dir, &spawned).await).is_some() {
                    return subscribe_recursive(
                        fs,
                        db,
                        pipes_dir,
                        caller,
                        spawned,
                        sub_id,
                        kind_filter,
                        tx,
                    )
                    .await;
                }
                send_inactive(tx, &sub_id).await?;
                return Ok(());
            }
        };

        loop {
            let event = match listener.next_event().await {
                Some(ev) => ev,
                None => {
                    let items = fs.read_new_from_queue(&db, &caller, &spawned).await?;
                    if !items.is_empty() {
                        send_items(tx, &sub_id, items).await?;
                    }
                    return Ok(());
                }
            };
            match event {
                SubscribeEvent::Row { message_kind: _ } => {
                    let items = fs.read_new_from_queue(&db, &caller, &spawned).await?;
                    if matches_filter(&items, kind_filter) {
                        matched = true;
                    }
                    if !items.is_empty() {
                        send_items(tx, &sub_id, items).await?;
                    }
                    if matched {
                        return Ok(());
                    }
                }
                SubscribeEvent::StreamEnd => {
                    let items = fs.read_new_from_queue(&db, &caller, &spawned).await?;
                    if !items.is_empty() {
                        send_items(tx, &sub_id, items).await?;
                    }
                    return Ok(());
                }
            }
        }
    })
}

fn matches_filter(items: &[QueueItem], filter: Option<RequestMessageKind>) -> bool {
    items.iter().any(|it| match filter {
        None => true,
        Some(k) => queue_item_kind(it) == k,
    })
}

fn queue_item_kind(item: &QueueItem) -> RequestMessageKind {
    match item {
        QueueItem::AssistantResponse { .. } => RequestMessageKind::AssistantResponse,
        QueueItem::ToolResponse { .. } => RequestMessageKind::ToolResponse,
        QueueItem::Notification { .. } => RequestMessageKind::AgentCompletionNotification,
        QueueItem::AgentCompletionRequest { .. } => RequestMessageKind::AgentCompletionRequest,
        QueueItem::FunctionExecutionRequest { .. } => RequestMessageKind::FunctionExecutionRequest,
        QueueItem::FunctionInventionRecursiveRequest { .. } => {
            RequestMessageKind::FunctionInventionRecursiveRequest
        }
    }
}

async fn send_items(
    tx: &mpsc::Sender<Result<ResponseItem, Error>>,
    sub_id: &str,
    items: Vec<QueueItem>,
) -> Result<(), Error> {
    let _ = tx
        .send(Ok(ResponseItem::Items {
            agent_id: sub_id.to_string(),
            items,
        }))
        .await;
    Ok(())
}

async fn send_inactive(
    tx: &mpsc::Sender<Result<ResponseItem, Error>>,
    sub_id: &str,
) -> Result<(), Error> {
    let _ = tx
        .send(Ok(ResponseItem::Inactive {
            agent_id: sub_id.to_string(),
        }))
        .await;
    Ok(())
}

struct EventStream {
    lines: tokio::io::Lines<BufReader<interprocess::local_socket::tokio::RecvHalf>>,
}

impl EventStream {
    async fn next_event(&mut self) -> Option<SubscribeEvent> {
        loop {
            let line = match self.lines.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => return None,
                Err(_) => return None,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<SubscribeEvent>(trimmed) {
                Ok(ev) => return Some(ev),
                Err(_) => return None,
            }
        }
    }
}

async fn try_connect_events(pipes_dir: &Path, spawned: &str) -> Option<EventStream> {
    let socket_path = pipes_dir.join(spawned).join("events.sock");
    let name = socket_path
        .to_fs_name::<GenericFilePath>()
        .ok()?
        .into_owned();
    let stream = interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .ok()?;
    let (read_half, _write_half) = stream.split();
    let reader = BufReader::new(read_half);
    Some(EventStream {
        lines: reader.lines(),
    })
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
