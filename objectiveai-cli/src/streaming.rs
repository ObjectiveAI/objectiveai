//! Subprocess bridge to the instance runner — restored shape of the
//! deleted `crate::api::stream_subprocess`, adapted to the bare-naked
//! `command::*::execute` contract.
//!
//! Streaming leaves spawn the cli binary as itself with the hidden
//! `instance` subcommand prepended to argv; the forwarded HTTP / MCP /
//! agent-id args ride as clap flags, the body as `--body <JSON>`, and
//! the subprocess's NDJSON stdout is consumed here.
//!
//! Per-chunk responsibilities split as follows:
//!
//! - **instance runner** owns: opening the WS, sending the conduit,
//!   coalesced log-file writing, emitting `LogStreamReady` once,
//!   per-agent named-pipe listeners.
//! - **leaf execute** owns: arg resolution, mapping the instance's
//!   NDJSON envelope into typed `ResponseItem`s, deciding whether to
//!   follow the stream or exit after the `LogStreamReady` handshake.
//!
//! ### Two modes
//!
//! - **`detach = true` (default)**: spawn the instance runner, wait
//!   for the first `LogStreamReady` notification, yield
//!   [`InstanceItem::Id`] with the response id, return. The instance
//!   runner child keeps running after the leaf's stream ends — the
//!   caller is expected to exit promptly so the orphan can take over
//!   the actual completion stream. Mirrors legacy `run_detached`.
//!
//! - **`detach = false` (follow)**: spawn the instance runner, yield
//!   [`InstanceItem::Id`] when the `LogStreamReady` notification
//!   arrives, then yield [`InstanceItem::Chunk`] for every chunk
//!   Notification the runner emits, until stdout EOF. Mirrors legacy
//!   `run` but as a streaming pipeline instead of an aggregate.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::ReceiverStream;

use objectiveai_sdk::cli::output::{
    LogStreamReady, Notification, NotificationValue, Output, TypedNotificationValue,
};

use crate::context::Context;
use crate::error::Error;

/// Re-export of the producer-side constant from [`crate::instance::api`]
/// so cli callers parse subprocess exit codes against the same value the
/// runner uses to signal a lost admission race.
pub use crate::instance::api::SLOT_TAKEN_EXIT_CODE;

/// Item yielded by [`instance_subprocess_stream`]. The leaf maps it to
/// its typed `ResponseItem`:
///
/// - `Id(s)`              → `ResponseItem::Id(s)`.
/// - `Chunk(v)`           → `ResponseItem::Chunk(serde_json::from_value(v)?)`.
pub enum InstanceItem {
    /// The `LogStreamReady` handshake — the instance runner has minted
    /// the response id and the log writer is fully wired up. Always
    /// yielded exactly once, before any `Chunk` items.
    Id(String),
    /// One chunk Notification from the instance runner's NDJSON stdout
    /// (its typed shape varies per endpoint, so it rides as a raw
    /// `serde_json::Value` through the `NotificationValue::Other`
    /// catch-all). Only yielded when `detach == false`.
    Chunk(serde_json::Value),
}

/// Spawn `objectiveai-cli instance <endpoint_path...> --body <JSON>`
/// and consume its NDJSON stdout as a stream.
///
/// `bind_agent_instance_hierarchy` — when the caller knows the full
/// agent id ahead of time (e.g. `agents message`'s continuation
/// fallback), pass it here to tell the instance runner to bind the
/// per-agent socket *before* opening the API stream. A lost race
/// surfaces as the `SLOT_TAKEN_EXIT_CODE` exit code, which this helper
/// maps to [`Error::CliStreamSlotTaken`]. `agents spawn` and the
/// function-create leaves always pass `None`.
///
/// `detach` — see module doc. `true` mirrors legacy `run_detached`;
/// `false` mirrors legacy `run` as a streaming pipeline.
pub fn instance_subprocess_stream(
    ctx: &Context,
    endpoint_path: &'static [&'static str],
    body: &(impl Serialize + ?Sized),
    bind_agent_instance_hierarchy: Option<String>,
    detach: bool,
) -> Pin<Box<dyn Stream<Item = Result<InstanceItem, Error>> + Send>> {
    let body_json = serde_json::to_string(body)
        .expect("body serialization to JSON should not fail for valid params");
    let cli_config = ctx.config.clone();
    let config_base_dir = cli_config.config_base_dir.clone();
    let fs = ctx.filesystem.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<InstanceItem, Error>>(16);

    tokio::spawn(async move {
        let result = run_subprocess(
            &cli_config,
            config_base_dir,
            fs,
            endpoint_path,
            &body_json,
            bind_agent_instance_hierarchy,
            detach,
            tx.clone(),
        )
        .await;
        if let Err(e) = result {
            // Best-effort: the receiver may already be dropped if the
            // consumer abandoned the stream. Ignore the send error.
            let _ = tx.send(Err(e)).await;
        }
    });

    Box::pin(ReceiverStream::new(rx))
}

#[allow(clippy::too_many_arguments)]
async fn run_subprocess(
    cli_config: &crate::run::Config,
    config_base_dir: Option<String>,
    fs: crate::filesystem::Client,
    endpoint_path: &'static [&'static str],
    body_json: &str,
    bind_agent_instance_hierarchy: Option<String>,
    detach: bool,
    tx: tokio::sync::mpsc::Sender<Result<InstanceItem, Error>>,
) -> Result<(), Error> {
    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let mut cmd = Command::new(&exe);
    cmd.arg("instance");

    push_forwarded_args(&mut cmd, cli_config, &fs).await?;

    for seg in endpoint_path {
        cmd.arg(seg);
    }

    cmd.args(["--body", body_json]);

    if let Some(id) = &bind_agent_instance_hierarchy {
        cmd.args(["--bind-agent-instance-hierarchy", id]);
    }

    if let Some(ref base) = config_base_dir {
        // Already passed via push_forwarded_args; kept consistent here so
        // the cli sees a single resolved value regardless of order.
        let _ = base;
    }

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // On Windows, detach the child from the parent's console + job
    // object so it survives when the parent exits.
    //   CREATE_NEW_PROCESS_GROUP (0x00000200): same flag legacy
    //     `api/detach.rs` used for the parent→child CLI re-exec.
    //   DETACHED_PROCESS (0x00000008): drop the inherited console so
    //     the child isn't taken down with the parent's console
    //     session — required because the parent leaf may exit as soon
    //     as it sees `LogStreamReady`, while instance-runner keeps
    //     streaming chunks for the rest of the request.
    #[cfg(windows)]
    if detach {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn("objectiveai-cli instance".into(), e))?;
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Capture stderr concurrently — mirror each line to this process's
    // stderr and remember the tail for the error path.
    let stderr_task = tokio::spawn(async move {
        let mut buf: Vec<String> = Vec::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            eprintln!("{l}");
            buf.push(l);
        }
        buf
    });

    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut handshake_seen = false;
    loop {
        let line = match stdout_lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                return Err(Error::Spawn(
                    "read instance-runner stdout".into(),
                    e,
                ));
            }
        };
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let out: Output = serde_json::from_str(trimmed).unwrap_or_else(|e| {
            panic!("instance-runner stdout produced a non-JSONL line: {trimmed}; parse error: {e}")
        });
        match out {
            Output::Notification(n) => {
                match handle_notification(n, detach, &tx).await {
                    HandleOutcome::HandshakeReturn => {
                        // `detach == true` path: yielded the Id, the
                        // instance-runner child keeps running orphaned.
                        // Drop the stderr task (the child is on its own
                        // now) and return without reaping.
                        drop(stderr_task);
                        return Ok(());
                    }
                    HandleOutcome::SawHandshake => {
                        handshake_seen = true;
                    }
                    HandleOutcome::Continue => {}
                    HandleOutcome::ConsumerGone => return Ok(()),
                }
            }
            Output::Error(_e) => {
                // Per-chunk errors from the runner are dropped at this
                // boundary — leaves return their own typed Result-stream
                // and don't have a `handle.emit()` to forward to.
                // TODO: surface as a stream-level Err once we settle on
                // a per-chunk error shape on `ItemStream`.
            }
        }
    }

    // Stdout EOF. Reap the child and decide whether the exit was clean.
    let stderr_buf = stderr_task.await.unwrap_or_default();
    let status = child
        .wait()
        .await
        .map_err(|e| Error::Spawn("wait for instance-runner".into(), e))?;

    if !status.success() {
        let tail: String = stderr_buf
            .iter()
            .rev()
            .take(20)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if status.code() == Some(SLOT_TAKEN_EXIT_CODE) {
            return Err(Error::CliStreamSlotTaken { stderr_tail: tail });
        }
        return Err(Error::CliStreamSubprocess {
            code: status.code().unwrap_or(-1),
            stderr_tail: tail,
        });
    }

    // Clean exit. If we never saw the handshake in detach mode, the
    // child died before the response id was minted — treat that as a
    // subprocess failure with no exit code distinction.
    if detach && !handshake_seen {
        let tail: String = stderr_buf
            .iter()
            .rev()
            .take(20)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(Error::CliStreamSubprocess {
            code: 0,
            stderr_tail: tail,
        });
    }

    Ok(())
}

enum HandleOutcome {
    /// `detach == true` and we just emitted the Id — return early.
    HandshakeReturn,
    /// `detach == false` and we just emitted the Id — keep reading.
    SawHandshake,
    /// Emitted a Chunk or dropped an unrelated Notification — keep
    /// reading.
    Continue,
    /// The receiver was dropped — the consumer abandoned the stream.
    ConsumerGone,
}

async fn handle_notification(
    n: Notification,
    detach: bool,
    tx: &tokio::sync::mpsc::Sender<Result<InstanceItem, Error>>,
) -> HandleOutcome {
    match n.value {
        NotificationValue::Typed(TypedNotificationValue::LogStreamReady(
            LogStreamReady { log_stream_ready },
        )) => {
            if tx.send(Ok(InstanceItem::Id(log_stream_ready))).await.is_err() {
                return HandleOutcome::ConsumerGone;
            }
            if detach {
                HandleOutcome::HandshakeReturn
            } else {
                HandleOutcome::SawHandshake
            }
        }
        NotificationValue::Other(map) => {
            if detach {
                // Pre-handshake chunks (shouldn't happen — the runner
                // emits LogStreamReady first) or post-handshake chunks
                // (handled by the HandshakeReturn early-return) — drop.
                return HandleOutcome::Continue;
            }
            let value = serde_json::Value::Object(map);
            if tx.send(Ok(InstanceItem::Chunk(value))).await.is_err() {
                return HandleOutcome::ConsumerGone;
            }
            HandleOutcome::Continue
        }
        _ => HandleOutcome::Continue,
    }
}

/// Resolve every instance-runner global flag from cli's `cli_config`,
/// env vars, and on-disk config — mirrors the `build_http_client`
/// precedence chain in [`crate::context`].
async fn push_forwarded_args(
    cmd: &mut Command,
    cli_config: &crate::run::Config,
    fs: &crate::filesystem::Client,
) -> Result<(), Error> {
    fn env(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    let mut config = fs
        .read_config()
        .await
        .map_err(Error::Filesystem)?;

    let address = env("OBJECTIVEAI_ADDRESS").or_else(|| {
        let api = config.api();
        crate::context::compose_url(api.get_address(), api.get_port())
    });
    if let Some(v) = address {
        cmd.args(["--api-address", &v]);
    }

    if let Some(v) = env("OBJECTIVEAI_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_objectiveai_authorization()
            .map(String::from)
    }) {
        cmd.args(["--objectiveai-authorization", &v]);
    }

    if let Some(v) = env("USER_AGENT").or_else(|| config.api().get_user_agent().map(String::from)) {
        cmd.args(["--user-agent", &v]);
    }

    if let Some(v) = env("X_TITLE").or_else(|| config.api().get_x_title().map(String::from)) {
        cmd.args(["--x-title", &v]);
    }

    if let Some(v) =
        env("HTTP_REFERER").or_else(|| config.api().get_http_referer().map(String::from))
    {
        cmd.args(["--http-referer", &v]);
    }

    if let Some(v) = env("GITHUB_AUTHORIZATION")
        .or_else(|| config.api().get_github_authorization().map(String::from))
    {
        cmd.args(["--github-authorization", &v]);
    }

    if let Some(v) = env("OPENROUTER_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_openrouter_authorization()
            .map(String::from)
    }) {
        cmd.args(["--openrouter-authorization", &v]);
    }

    let mcp_auth_json = env("MCP_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_mcp_authorization()
            .map(|m| serde_json::to_string(m).expect("encoding String→String map"))
    });
    if let Some(v) = mcp_auth_json {
        cmd.args(["--mcp-authorization", &v]);
    }

    if let Some(v) =
        env("VIEWER_SIGNATURE").or_else(|| config.viewer().get_signature().map(String::from))
    {
        cmd.args(["--viewer-signature", &v]);
    }

    let viewer_address = env("VIEWER_ADDRESS").or_else(|| {
        let viewer = config.viewer();
        crate::context::compose_url(viewer.get_address(), viewer.get_port())
    });
    if let Some(v) = viewer_address {
        cmd.args(["--viewer-address", &v]);
    }

    if let Some(v) = env("COMMIT_AUTHOR_NAME")
        .or_else(|| config.api().get_commit_author_name().map(String::from))
    {
        cmd.args(["--commit-author-name", &v]);
    }

    if let Some(v) = env("COMMIT_AUTHOR_EMAIL")
        .or_else(|| config.api().get_commit_author_email().map(String::from))
    {
        cmd.args(["--commit-author-email", &v]);
    }

    cmd.args([
        "--objectiveai-agent-instance-hierarchy",
        &cli_config.agent_instance_hierarchy,
    ]);

    if let Some(ref v) = cli_config.mcp_session_id {
        cmd.args(["--mcp-session-id", v]);
    }

    if let Some(ref base) = cli_config.config_base_dir {
        cmd.args(["--config-base-dir", base]);
    }

    Ok(())
}
