//! `agents message` — bare-naked handler.
//!
//! Deliver a rich-content message to a running spawned agent. If the
//! per-agent socket (`${config_base_dir}/pipes/<full_id>/socket`) is
//! bound and acks the line, return [`Response::Delivered`]. If the
//! socket is unreachable or refuses to ack, fall back to continuing
//! the agent's most recent completion via its stored continuation
//! token and return [`Response::Queued`] with the new turn's response
//! id.
//!
//! Retry on [`Error::CliStreamSlotTaken`] — another caller's instance
//! runner currently owns the per-agent socket. Each retry first
//! re-runs `try_pipe_delivery` (cheap), so once the winner has bound
//! and is serving, the next pass delivers via the pipe and never
//! re-spawns. Unbounded: the only way to escape `SlotTaken` is for
//! the winner to release the socket, at which point we win or deliver
//! via the now-live pipe.

use std::path::Path;
use std::time::Duration;

use futures::StreamExt;
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use objectiveai_sdk::agent::completions::message::{
    Message, PipeAck, RichContent, UserMessage,
};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::message::{
    Request, RequestMessage, Response,
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

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
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

    loop {
        match handle_once(
            ctx,
            &full_id,
            &request.agent_instance,
            content.clone(),
            request.seed,
        )
        .await
        {
            Err(Error::CliStreamSlotTaken { .. }) => continue,
            other => return other,
        }
    }
}

async fn handle_once(
    ctx: &Context,
    full_id: &str,
    agent_instance: &str,
    content: RichContent,
    seed: Option<i64>,
) -> Result<Response, Error> {
    // Try live delivery first. Any failure here triggers the
    // continuation fallback — pipe errors are never surfaced as fatal.
    match try_pipe_delivery(ctx, full_id, &content).await {
        Ok(()) => Ok(Response::Delivered {
            agent_id: agent_instance.to_string(),
        }),
        Err(_) => fallback_via_continuation(ctx, full_id, agent_instance, content, seed).await,
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
/// resume via continuation, return `Response::Queued` with the new
/// turn's response id once the instance runner's `LogStreamReady`
/// handshake fires.
async fn fallback_via_continuation(
    ctx: &Context,
    full_id: &str,
    agent_instance: &str,
    content: RichContent,
    cli_seed: Option<i64>,
) -> Result<Response, Error> {
    // The plan's hard rule: a non-existent agent id does NOT auto-spawn.
    // Walk-back is in the filesystem helper — it tries each request
    // newest-first and returns the most recent one whose continuation
    // file exists, only erroring if NONE have one.
    let latest = match ctx.filesystem.read_latest_continuation(full_id).await? {
        LatestContinuationOutcome::Found(l) => l,
        LatestContinuationOutcome::NoRequests => {
            return Err(Error::AgentNoPriorRequest {
                agent_instance_hierarchy: full_id.to_string(),
            });
        }
        LatestContinuationOutcome::NoContinuationsFound { request_count } => {
            return Err(Error::AgentNoContinuation {
                agent_instance_hierarchy: full_id.to_string(),
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
    // work. `stream: false` → yield one `Id` then exit; the runner
    // child keeps running orphaned and drives the completion.
    let mut stream = instance_subprocess_stream(
        ctx,
        crate::instance::request::InstanceEndpoint::AgentsSpawn(params),
        Some(full_id.to_string()),
        false,
    );
    match stream.next().await {
        Some(Ok(InstanceItem::Id(new_response_id))) => Ok(Response::Queued {
            agent_id: agent_instance.to_string(),
            response_id: new_response_id,
        }),
        Some(Ok(InstanceItem::Chunk(_))) => {
            unreachable!("stream=false yields only Id before exit")
        }
        Some(Err(e)) => Err(e),
        None => Err(Error::CliStreamSubprocess {
            code: 0,
            stderr_tail: String::new(),
        }),
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
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
