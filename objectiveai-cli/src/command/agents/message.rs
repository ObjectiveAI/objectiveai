//! `agents message` — bare-naked chunk-or-id streaming handler.
//!
//! Deliver a rich-content message to a running spawned agent. If the
//! per-agent socket (`${config_base_dir}/pipes/<full_id>/socket`) is
//! bound and acks the line, yield a single [`ResponseItem::Delivered`]
//! item and end. If the socket is unreachable or refuses to ack, fall
//! back to continuing the agent's most recent completion via its
//! stored continuation token. The streamed item shape then depends on
//! `dangerous_advanced.stream`:
//!
//! - **`None | Some(false)` (default)** — yield a single
//!   [`ResponseItem::Queued`] carrying the new turn's `response_id`,
//!   then end. The instance runner child keeps running orphaned and
//!   drives the completion to completion (same shape as the legacy
//!   `agents message` behaviour — preserved unchanged for callers who
//!   don't set the flag).
//! - **`Some(true)`** — yield the same [`ResponseItem::Queued`]
//!   first, then one [`ResponseItem::Chunk`] per chunk Notification
//!   the runner emits, until the runner's stdout EOFs. The parent
//!   cli stays attached and `child.wait()`s the runner before its
//!   own exit, so `collect_stream` returning implies process exit
//!   — the synchronisation primitive integration tests need to avoid
//!   leaked instance-runner processes.
//!
//! Retry on [`Error::CliStreamSlotTaken`] is unchanged from the
//! legacy handler — another caller's instance runner currently owns
//! the per-agent socket, so we re-run `try_pipe_delivery` (cheap);
//! once the winner has bound and is serving, the next pass delivers
//! via the pipe and never re-spawns. Unbounded — the only way to
//! escape `SlotTaken` is for the winner to release the socket, at
//! which point we win or deliver via the now-live pipe.

use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use futures::{Stream, StreamExt};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use objectiveai_sdk::agent::completions::message::{
    Message, PipeAck, RichContent, UserMessage,
};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::cli::command::agents::message::{
    Request, RequestMessage, ResponseItem,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::logs::LatestContinuationOutcome;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

/// Connect-attempt deadline + per-line ack deadline. The pipe is
/// local; the live target is either already accepting or it's not
/// coming back in this lifetime.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(1000);
const ACK_TIMEOUT: Duration = Duration::from_millis(5000);

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // Compose the full lineage. `parent` from the request when set,
    // otherwise the cli's own `Config.agent_instance_hierarchy` (the
    // cli's "caller" position). Matches what
    // `LogWriter::with_caller_agent_instance_hierarchy` stores in
    // `messages.agent_instance_hierarchy` and what
    // `instance/streaming` binds the per-agent socket at
    // (`pipes/<parent>/<instance>/socket`).
    let parent = request
        .parent_agent_instance_hierarchy
        .as_deref()
        .unwrap_or(&ctx.config.agent_instance_hierarchy);
    let full_id = format!("{parent}/{}", request.agent_instance);
    let content = resolve_message(request.message)?;
    let stream_flag = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    let seed = request.seed;

    // Retry loop wraps the pipe-delivery + continuation-fallback
    // decision. Slot-taken errors at the continuation-fallback level
    // trigger a re-run from the top so the next pass can deliver via
    // the now-live pipe; everything else propagates.
    loop {
        // Try live delivery first. Any failure here falls through to
        // the continuation fallback — pipe errors are never surfaced
        // as fatal.
        if try_pipe_delivery(ctx, &full_id, &content).await.is_ok() {
            let item = ResponseItem::Delivered {
                agent_instance_hierarchy: full_id.clone(),
            };
            return Ok(Box::pin(futures::stream::once(async move { Ok(item) })));
        }

        match start_continuation_stream(
            ctx,
            full_id.clone(),
            content.clone(),
            seed,
            stream_flag,
        )
        .await
        {
            Ok(s) => return Ok(s),
            Err(Error::CliStreamSlotTaken { .. }) => continue,
            Err(e) => return Err(e),
        }
    }
}

/// Connect to `${config_base_dir}/pipes/<full_id>/socket`, write one
/// NDJSON `RichContent` line, and read back one `PipeAck` line.
/// Returns `Ok(())` only on `PipeAck::Ok`; any IO failure, timeout,
/// parse error, or `PipeAck::Error` is reported as `Err`.
async fn try_pipe_delivery(
    ctx: &Context,
    full_id: &str,
    content: &RichContent,
) -> Result<(), PipeError> {
    let base_dir = ctx
        .config
        .config_base_dir
        .as_deref()
        .ok_or(PipeError::NoBaseDir)?;
    let socket_path = Path::new(base_dir).join("pipes").join(full_id).join("socket");
    let name = socket_path
        .clone()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| PipeError::AddressInvalid(e.to_string()))?;

    let stream = tokio::time::timeout(
        CONNECT_TIMEOUT,
        interprocess::local_socket::tokio::Stream::connect(name),
    )
    .await
    .map_err(|_| PipeError::Timeout)?
    .map_err(|e| PipeError::Connect(e.to_string()))?;

    let (read_half, mut write_half) = stream.split();

    let line = serde_json::to_string(content).expect("RichContent serializes");
    write_half
        .write_all(line.as_bytes())
        .await
        .map_err(|e| PipeError::Write(e.to_string()))?;
    write_half
        .write_all(b"\n")
        .await
        .map_err(|e| PipeError::Write(e.to_string()))?;
    write_half
        .flush()
        .await
        .map_err(|e| PipeError::Write(e.to_string()))?;

    let mut ack_line = String::new();
    let mut reader = BufReader::new(read_half);
    let bytes = tokio::time::timeout(ACK_TIMEOUT, reader.read_line(&mut ack_line))
        .await
        .map_err(|_| PipeError::Timeout)?
        .map_err(|e| PipeError::Read(e.to_string()))?;
    if bytes == 0 {
        return Err(PipeError::Closed);
    }

    let ack: PipeAck = serde_json::from_str(ack_line.trim())
        .map_err(|e| PipeError::AckParse(e.to_string()))?;
    match ack {
        PipeAck::Ok => Ok(()),
        PipeAck::Error { message } => Err(PipeError::AckError(message)),
    }
}

#[derive(Debug)]
enum PipeError {
    NoBaseDir,
    AddressInvalid(String),
    Timeout,
    Connect(String),
    Write(String),
    Read(String),
    Closed,
    AckParse(String),
    AckError(String),
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeError::NoBaseDir => write!(f, "config_base_dir is not set"),
            PipeError::AddressInvalid(e) => write!(f, "pipe address: {e}"),
            PipeError::Timeout => write!(f, "pipe timeout"),
            PipeError::Connect(e) => write!(f, "pipe connect: {e}"),
            PipeError::Write(e) => write!(f, "pipe write: {e}"),
            PipeError::Read(e) => write!(f, "pipe read: {e}"),
            PipeError::Closed => write!(f, "pipe closed before ack"),
            PipeError::AckParse(e) => write!(f, "pipe ack parse: {e}"),
            PipeError::AckError(e) => write!(f, "pipe ack reported error: {e}"),
        }
    }
}

/// Live delivery failed — look up the agent's most recent completion,
/// resume via continuation, peek the spawned instance runner's first
/// item to extract the new turn's `response_id`, and produce the
/// `Queued` (plus optional `Chunk`s) stream.
///
/// `stream_flag = false`: the inner `instance_subprocess_stream` was
/// opened detached, so its child instance-runner keeps running
/// orphaned after we yield the single `Queued` item and drop the
/// remainder. `stream_flag = true`: we chain the remaining
/// `InstanceItem::Chunk`s through, and the inner stream ends only
/// when the parent cli has `child.wait()`ed instance-runner — so
/// `collect_stream` returning genuinely implies process exit.
async fn start_continuation_stream(
    ctx: &Context,
    full_id: String,
    content: RichContent,
    cli_seed: Option<i64>,
    stream_flag: bool,
) -> Result<ItemStream, Error> {
    // The plan's hard rule: a non-existent agent id does NOT auto-spawn.
    // Walk-back is in the filesystem helper — it tries each request
    // newest-first and returns the most recent one whose continuation
    // file exists, only erroring if NONE have one.
    let latest = match ctx.filesystem.read_latest_continuation(&full_id).await? {
        LatestContinuationOutcome::Found(l) => l,
        LatestContinuationOutcome::NoRequests => {
            return Err(Error::AgentNoPriorRequest {
                agent_instance_hierarchy: full_id,
            });
        }
        LatestContinuationOutcome::NoContinuationsFound { request_count } => {
            return Err(Error::AgentNoContinuation {
                agent_instance_hierarchy: full_id,
                request_count,
            });
        }
    };

    let params = AgentCompletionCreateParams {
        messages: vec![Message::User(UserMessage {
            content,
            name: None,
        })],
        provider: latest.provider,
        agent: latest.agent,
        response_format: latest.response_format,
        // cli flag overrides the original's seed when set.
        seed: cli_seed.or(latest.seed),
        stream: Some(true),
        continuation: Some(latest.continuation),
    };

    // Eager admission probe — claim the per-agent socket before opening
    // the API stream so a racing peer's instance runner gets the
    // SLOT_TAKEN exit immediately rather than after some wasted API
    // work. `stream_flag` controls only the parent cli's attachment
    // shape; the instance runner's API request always streams.
    let mut sub = instance_subprocess_stream(
        ctx,
        crate::instance::request::InstanceEndpoint::AgentsSpawn(params),
        Some(full_id.clone()),
        stream_flag,
    );

    // Peek the first item: it must be the runner's `LogStreamReady`
    // `Id`. Any error here (including `CliStreamSlotTaken`) propagates
    // to the caller so the outer retry loop can re-run.
    let new_response_id = match sub.next().await {
        Some(Ok(InstanceItem::Id(id))) => id,
        Some(Ok(InstanceItem::Chunk(_))) => {
            unreachable!("instance runner emits Id before any Chunk")
        }
        Some(Err(e)) => return Err(e),
        None => {
            return Err(Error::CliStreamSubprocess {
                code: 0,
                stderr_tail: String::new(),
            });
        }
    };

    let queued = ResponseItem::Queued {
        agent_instance_hierarchy: full_id,
        response_id: new_response_id,
    };
    let head = futures::stream::once(async move { Ok(queued) });

    if stream_flag {
        // Chain every subsequent `InstanceItem::Chunk` as
        // `ResponseItem::Chunk`. The inner stream ends when the
        // instance runner's stdout EOFs — i.e. after the cli has
        // `child.wait()`ed it. Returning from `collect_stream` on the
        // caller side therefore implies the runner exited.
        let tail = sub.map(|item| match item? {
            InstanceItem::Id(_) => {
                unreachable!("only one Id per instance subprocess stream")
            }
            InstanceItem::Chunk(value) => serde_json::from_value::<
                AgentCompletionChunk,
            >(value)
            .map(ResponseItem::Chunk)
            .map_err(Error::InlineJson),
        });
        Ok(Box::pin(head.chain(tail)))
    } else {
        // Detach the rest. `instance_subprocess_stream` opened with
        // `stream = false` already left the instance runner orphaned;
        // dropping `sub` here just stops draining its output (which
        // is now going to a log file, not back to us).
        drop(sub);
        Ok(Box::pin(head))
    }
}

fn resolve_message(message: RequestMessage) -> Result<RichContent, Error> {
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
