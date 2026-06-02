//! `agents read subscribe <sub-id> [--kind <message-kind>]` — block
//! on the spawned agent's next queue event, with strict
//! out-of-sync-prevention semantics.
//!
//! ## Algorithm (the ordering matters)
//!
//! ```text
//! SUBSCRIBE(sub_id, kind_filter):
//!   listener := try_connect(events.sock for sub_id)
//!   items := read_new_from_queue(caller, spawned)
//!   if items not empty: emit AgentItems
//!   matched := kind_filter.is_none() OR any(item.kind == filter)
//!   if matched: return
//!
//!   if listener is None:
//!     listener := try_connect(events.sock for sub_id)  // 2nd attempt
//!     if listener is Some:
//!       return SUBSCRIBE(sub_id, kind_filter)          // recurse
//!     emit Inactive { agent_id: sub_id }
//!     return
//!
//!   loop:
//!     event := listener.recv()
//!     match event:
//!       Row { .. }:
//!         items := read_new_from_queue(caller, spawned)
//!         if items not empty: emit AgentItems
//!         if matched: return
//!       StreamEnd:
//!         items := read_new_from_queue(caller, spawned)
//!         if items not empty: emit AgentItems
//!         return
//! ```
//!
//! INVARIANT 1: the listener is opened BEFORE the first DB read, so
//! any row inserted between "function entry" and "drain returned" is
//! guaranteed to be buffered in the broadcast channel and picked up
//! by the loop.
//!
//! INVARIANT 2: if the listener was missing on first try, we attempt
//! ONE more connect before declaring the agent inactive — catches the
//! tight race where the pipe appears in between. On second-attempt
//! success we recurse so the new call re-establishes invariant 1
//! from a fresh starting point.

use std::path::PathBuf;
use std::pin::Pin;

use clap::{Args, ValueEnum};
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use objectiveai_sdk::cli::output::notification::agents::{AgentItems, Inactive};
use objectiveai_sdk::cli::output::{Handle, Notification, Output};
use crate::filesystem::db::schema::MessageKind;
use crate::filesystem::logs::{SubscribeEvent, queue::QueueItem};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Args)]
pub struct CommandArgs {
    /// Sub-id (lineage-relative) of the spawned agent to subscribe
    /// to. The caller prefix (`cli_config.agent_instance_hierarchy`) is prepended
    /// internally — same shape as `agents read pending`.
    pub agent_instance_hierarchy: String,

    /// Optional filter. When supplied, `subscribe` returns only when
    /// a queue item of this kind is observed (or the stream ends).
    /// Without it, any queue event satisfies the wait.
    ///
    /// Notes on initial drain semantics: ALL drained items are
    /// emitted as a single `AgentItems` regardless of `--kind`; the
    /// kind filter only governs whether the command exits or keeps
    /// waiting.
    #[arg(long = "kind", value_enum)]
    pub kind: Option<MessageKindArg>,
}

/// Clap-friendly mirror of [`MessageKind`]. We keep the canonical
/// enum in the SDK free of clap deps; this maps 1:1.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum MessageKindArg {
    AgentCompletionRequest,
    FunctionExecutionRequest,
    FunctionInventionRecursiveRequest,
    AgentCompletionNotification,
    AssistantResponse,
    ToolResponse,
}

impl From<MessageKindArg> for MessageKind {
    fn from(a: MessageKindArg) -> Self {
        match a {
            MessageKindArg::AgentCompletionRequest => MessageKind::AgentCompletionRequest,
            MessageKindArg::FunctionExecutionRequest => MessageKind::FunctionExecutionRequest,
            MessageKindArg::FunctionInventionRecursiveRequest => {
                MessageKind::FunctionInventionRecursiveRequest
            }
            MessageKindArg::AgentCompletionNotification => MessageKind::AgentCompletionNotification,
            MessageKindArg::AssistantResponse => MessageKind::AssistantResponse,
            MessageKindArg::ToolResponse => MessageKind::ToolResponse,
        }
    }
}

pub async fn handle(
    args: CommandArgs,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let kind_filter = args.kind.map(MessageKind::from);
    let client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        None::<String>,
        None::<String>,
    );
    let caller = cli_config.agent_instance_hierarchy.clone();
    let spawned = format!("{caller}/{}", args.agent_instance_hierarchy);
    let pipes_dir = client.pipes_dir();
    subscribe_recursive(
        client,
        pipes_dir,
        caller,
        spawned,
        args.agent_instance_hierarchy,
        kind_filter,
        handle,
    )
    .await
}

/// The recursive driver. Boxed because clippy/rustc demand it for
/// async-recursion.
fn subscribe_recursive(
    client: crate::filesystem::Client,
    pipes_dir: PathBuf,
    caller: String,
    spawned: String,
    sub_id: String,
    kind_filter: Option<MessageKind>,
    handle: &Handle,
) -> Pin<Box<dyn Future<Output = Result<(), crate::error::Error>> + Send + '_>> {
    Box::pin(async move {
        // INVARIANT 1: open the listener BEFORE the first DB query.
        let listener = try_connect_events(&pipes_dir, &spawned).await;

        // Initial drain. ALL items emitted; kind filter only governs
        // the "did we find a match" decision.
        let items = client.read_new_from_queue(&caller, &spawned).await?;
        let mut matched = matches_filter(&items, kind_filter);
        if !items.is_empty() {
            emit_items(handle, &sub_id, items).await;
        }
        if matched {
            return Ok(());
        }

        let mut listener = match listener {
            Some(l) => l,
            None => {
                // INVARIANT 2: second listener-open attempt before
                // declaring inactive. Catches the race where the
                // pipe appeared between function entry and now.
                if let Some(_l2) = try_connect_events(&pipes_dir, &spawned).await {
                    return subscribe_recursive(
                        client,
                        pipes_dir,
                        caller,
                        spawned,
                        sub_id,
                        kind_filter,
                        handle,
                    )
                    .await;
                }
                emit_inactive(handle, &sub_id).await;
                return Ok(());
            }
        };

        // Listener is open — block on real event delivery.
        loop {
            let event = match listener.next_event().await {
                Some(ev) => ev,
                None => {
                    // Pipe closed without `StreamEnd` (writer error
                    // or abnormal exit). Final drain attempt, then
                    // return — no more events possible.
                    let items = client.read_new_from_queue(&caller, &spawned).await?;
                    if !items.is_empty() {
                        emit_items(handle, &sub_id, items).await;
                    }
                    return Ok(());
                }
            };
            match event {
                SubscribeEvent::Row { message_kind: _ } => {
                    let items = client.read_new_from_queue(&caller, &spawned).await?;
                    if matches_filter(&items, kind_filter) {
                        matched = true;
                    }
                    if !items.is_empty() {
                        emit_items(handle, &sub_id, items).await;
                    }
                    if matched {
                        return Ok(());
                    }
                }
                SubscribeEvent::StreamEnd => {
                    let items = client.read_new_from_queue(&caller, &spawned).await?;
                    if !items.is_empty() {
                        emit_items(handle, &sub_id, items).await;
                    }
                    return Ok(());
                }
            }
        }
    })
}

/// True iff the drained `items` slice contains at least one element
/// matching the filter. `None` filter ≡ "any item satisfies." With an
/// empty `items` slice this always returns false — that's the
/// semantic the user specified ("waits if there are no unread of
/// the specified kind," which when the kind is unconstrained means
/// "waits if there are no unread").
fn matches_filter(items: &[QueueItem], filter: Option<MessageKind>) -> bool {
    items.iter().any(|it| match filter {
        None => true,
        Some(k) => queue_item_kind(it) == k,
    })
}

/// Map the typed [`QueueItem`] back to its [`MessageKind`]. The two
/// share variant names by construction (see `queue_item_from_row`
/// in the SDK).
fn queue_item_kind(item: &QueueItem) -> MessageKind {
    match item {
        QueueItem::AssistantResponse { .. } => MessageKind::AssistantResponse,
        QueueItem::ToolResponse { .. } => MessageKind::ToolResponse,
        QueueItem::Notification { .. } => MessageKind::AgentCompletionNotification,
        QueueItem::AgentCompletionRequest { .. } => MessageKind::AgentCompletionRequest,
        QueueItem::FunctionExecutionRequest { .. } => MessageKind::FunctionExecutionRequest,
        QueueItem::FunctionInventionRecursiveRequest { .. } => {
            MessageKind::FunctionInventionRecursiveRequest
        }
    }
}

async fn emit_items(handle: &Handle, sub_id: &str, items: Vec<QueueItem>) {
    Output::Notification(Notification {
        value: (AgentItems {
            agent_id: sub_id.to_string(),
            items,
        })
        .into(),
    })
    .emit(handle)
    .await;
}

async fn emit_inactive(handle: &Handle, sub_id: &str) {
    Output::Notification(Notification {
        value: (Inactive {
            agent_id: sub_id.to_string(),
        })
        .into(),
    })
    .emit(handle)
    .await;
}

/// Wrapper around the AF_UNIX client that owns the read half of the
/// connection plus a line buffer, and exposes `next_event` for the
/// loop. Constructed only via [`try_connect_events`].
struct EventStream {
    lines: tokio::io::Lines<BufReader<interprocess::local_socket::tokio::RecvHalf>>,
}

impl EventStream {
    /// Read one NDJSON line and parse it as a [`SubscribeEvent`].
    /// Returns `None` on EOF / read error / parse error — every
    /// terminal condition the loop should treat the same way.
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

/// Try to connect to `${pipes_dir}/<spawned>/events.sock` as a
/// client. Returns `None` if the socket doesn't exist or the connect
/// fails for any reason — the caller decides whether to recurse,
/// emit `Inactive`, or proceed with the loop.
async fn try_connect_events(pipes_dir: &std::path::Path, spawned: &str) -> Option<EventStream> {
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
