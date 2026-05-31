//! `agents message` — deliver a rich-content message to a running
//! spawned agent. If the per-agent socket
//! (`${config_base_dir}/pipes/<full_id>/socket`) is bound + acks the
//! line, we exit emitting [`MessageDelivered`]. If the socket is
//! unreachable or refuses to ack, we fall back to continuing the
//! agent's most recent completion via its stored continuation, and
//! exit emitting [`MessageQueued`] — same envelope shape as a fresh
//! spawn, but reusing the original `agent_id` instead of minting a new
//! one.
//!
//! Closes the gap between `agents spawn` (fire-and-forget) and
//! `agents read pending` (drain-the-mailbox): there was no way for the
//! parent to **push** a new turn into an existing conversation.

use std::time::Duration;

use clap::Args;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use objectiveai_sdk::agent::completions::message::{
    Message, PipeAck, RichContent, UserMessage,
};
use objectiveai_sdk::cli::output::{
    Handle, MessageDelivered, Notification, NotificationValue, Output,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Connect-attempt deadline + per-line ack deadline. The pipe is
/// local; the live target is either already accepting or it's not
/// coming back in this lifetime.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(1000);
const ACK_TIMEOUT: Duration = Duration::from_millis(5000);

/// How the message body is provided. Mirrors `spawn::PromptSource` —
/// same five long flags, same clap group convention — but resolves to
/// a single [`RichContent`] (the body of one user turn) instead of a
/// `Vec<Message>`.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct MessageSource {
    /// Plain text — becomes a `RichContent::Text(<text>)` body.
    #[arg(long)]
    simple: Option<String>,
    /// Inline JSON `RichContent`.
    #[arg(long)]
    inline: Option<String>,
    /// Path to a JSON file containing a `RichContent`.
    #[arg(long)]
    file: Option<std::path::PathBuf>,
    /// Inline Python code that produces a `RichContent`.
    #[arg(long)]
    python_inline: Option<String>,
    /// Path to a Python file that produces a `RichContent`.
    #[arg(long)]
    python_file: Option<std::path::PathBuf>,
}

impl MessageSource {
    fn resolve(self) -> Result<RichContent, crate::error::Error> {
        crate::source_resolver::resolve_source(
            self.simple,
            self.inline,
            self.file,
            self.python_inline,
            self.python_file,
            RichContent::Text,
        )
    }
}

#[derive(Args)]
pub struct CommandArgs {
    /// Sub-id (lineage-relative) of the target agent. The caller
    /// prefix (`OBJECTIVEAI_AGENT_ID`, defaulting to `"cli"`) is
    /// prepended internally — same convention as
    /// `agents read pending`.
    pub agent_id: String,
    #[command(flatten)]
    pub message: MessageSource,
    /// Seed for deterministic mock responses on the continuation
    /// fallback. **Ignored on the live-delivery path** — the pipe
    /// wire is `RichContent` only; the running agent's seed is set
    /// at spawn time. On the fallback path, this overrides the
    /// original request's seed if both are set.
    #[arg(long)]
    pub seed: Option<i64>,
}

pub async fn handle(
    args: CommandArgs,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    // Glue the caller's lineage onto the user-supplied sub-id —
    // matches what `LogWriter::with_caller_agent_id` stores in
    // `messages.agent_id` and what `streaming.rs` binds the per-agent
    // socket at (`pipes/<caller>/<sub-id>/socket`). Same convention
    // every other agent-lookup command (e.g. `agents read pending`)
    // uses on the read side.
    let caller = &cli_config.agent_id;
    let full_agent_id = format!("{caller}/{}", args.agent_id);
    let content = args.message.resolve()?;

    // Try live delivery first. Any failure here triggers the
    // continuation fallback — we never surface pipe errors as fatal.
    match try_pipe_delivery(cli_config, &full_agent_id, &content).await {
        Ok(()) => {
            Output::Notification(Notification {
                agent_id: None,
                value: MessageDelivered { agent_id: full_agent_id }.into(),
            })
            .emit(handle)
            .await;
            Ok(())
        }
        Err(_) => fall_back_to_continuation(cli_config, &full_agent_id, content, args.seed, handle).await,
    }
}

/// Connect to `${config_base_dir}/pipes/<full_agent_id>/socket`,
/// write one NDJSON `RichContent` line, and read back one `PipeAck`
/// line. Returns `Ok(())` only on `PipeAck::Ok`; any IO failure,
/// timeout, parse error, or `PipeAck::Error` is reported as `Err`.
async fn try_pipe_delivery(
    cli_config: &crate::Config,
    full_agent_id: &str,
    content: &RichContent,
) -> Result<(), PipeError> {
    let base_dir = cli_config
        .config_base_dir
        .as_deref()
        .ok_or_else(|| PipeError::NoBaseDir)?;
    let socket_path = std::path::Path::new(base_dir)
        .join("pipes")
        .join(full_agent_id)
        .join("socket");
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

    let line = serde_json::to_string(content)
        .expect("RichContent serializes");
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
/// resume via continuation, emit `MessageQueued` if cli-stream's
/// handshake fires.
async fn fall_back_to_continuation(
    cli_config: &crate::Config,
    full_agent_id: &str,
    content: RichContent,
    cli_seed: Option<i64>,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let fs_client = objectiveai_sdk::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );

    // The plan's hard rule: a non-existent agent id does NOT auto-spawn.
    // Walk-back is in the SDK helper — it tries each request newest-
    // first and returns the most recent one whose continuation file
    // exists, only erroring if NONE have one.
    use objectiveai_sdk::filesystem::logs::LatestContinuationOutcome;
    let latest = match fs_client.read_latest_continuation(full_agent_id).await? {
        LatestContinuationOutcome::Found(l) => l,
        LatestContinuationOutcome::NoRequests => {
            return Err(crate::error::Error::AgentNoPriorRequest {
                agent_id: full_agent_id.to_string(),
            });
        }
        LatestContinuationOutcome::NoContinuationsFound { request_count } => {
            return Err(crate::error::Error::AgentNoContinuation {
                agent_id: full_agent_id.to_string(),
                request_count,
            });
        }
    };

    let params = objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages: vec![Message::User(UserMessage { content, name: None })],
        provider: latest.provider,
        agent: latest.agent,
        response_format: latest.response_format,
        // cli flag overrides the original's seed when set.
        seed: cli_seed.or(latest.seed),
        stream: Some(true),
        continuation: Some(latest.continuation),
    };

    let full_agent_id_for_notif = full_agent_id.to_string();
    crate::api::stream_subprocess::run_detached_with(
        cli_config,
        &["agents", "spawn"],
        &params,
        handle,
        move |new_response_id| {
            // The original agent_id stays the same across a
            // continuation; we surface the NEW response_id (cli-stream
            // emits its root log id via `LogStreamReady`) so callers
            // can correlate the freshly-started continuation turn.
            NotificationValue::MessageQueued(objectiveai_sdk::cli::output::MessageQueued {
                agent_id: full_agent_id_for_notif,
                response_id: new_response_id,
            })
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without a bound socket the helper should error rather than
    /// hang. The OS-specific bind path used by cli-stream lives in
    /// `objectiveai-cli-stream/src/pipes.rs` and is exercised end-to-end
    /// there; the per-line `PipeAck` wire shape is locked in by
    /// `objectiveai-sdk` round-trip tests.
    #[tokio::test]
    async fn pipe_delivery_unreachable_when_no_socket() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cli_config = crate::Config {
            config_set_forbidden: false,
            config_base_dir: Some(tmp.path().to_string_lossy().into_owned()),
            commit_author_name: None,
            commit_author_email: None,
            github_authorization: None,
            agent_id: "cli".to_string(),
            mcp_session_id: None,
            mcp: false,
        };
        let result = try_pipe_delivery(
            &cli_config,
            "cli/never-existed",
            &RichContent::Text("hi".into()),
        )
        .await;
        assert!(matches!(result, Err(_)), "expected pipe error, got {result:?}");
    }
}
