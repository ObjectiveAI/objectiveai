//! Subprocess bridge to the instance runner.
//!
//! Streaming leaves spawn the cli binary as `objectiveai-cli instance`;
//! the parent inherits one end of an anonymous pipe into the child,
//! writes a magic-header + JSON [`InstanceRequest`] blob, and consumes
//! the child's NDJSON stdout as a stream.
//!
//! Per-chunk responsibilities split as follows:
//!
//! - **instance runner** owns: opening the WS, sending the conduit,
//!   coalesced log-file writing, emitting `LogStreamReady` once,
//!   per-agent named-pipe listeners.
//! - **leaf execute** owns: arg resolution, mapping the instance's
//!   NDJSON envelope into typed `ResponseItem`s, deciding whether to
//!   wait on the stream or detach after the `LogStreamReady`
//!   handshake.
//!
//! ### The `stream` parameter
//!
//! - **`stream = false` (default)**: spawn the instance runner detached,
//!   wait for the first `LogStreamReady` notification, yield
//!   [`InstanceItem::Id`] with the response id, return. The instance
//!   runner child keeps running orphaned and drives the completion to
//!   completion on its own. Mirrors legacy `run_detached`.
//!
//! - **`stream = true`**: spawn the instance runner, yield
//!   [`InstanceItem::Id`] when the `LogStreamReady` notification
//!   arrives, then yield [`InstanceItem::Chunk`] for every chunk
//!   Notification the runner emits, until stdout EOF. Mirrors legacy
//!   `run` but as a streaming pipeline instead of an aggregate.

use std::pin::Pin;

use futures::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::ReceiverStream;

use crate::context::Context;
use crate::error::Error;
use crate::instance::InstanceEmission;
use crate::instance::handshake::{self, PIPE_ENV};
use crate::instance::request::{HttpConfig, InstanceEndpoint, InstanceRequest, PipeConfig};

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
    /// catch-all). Only yielded when `stream == true`.
    Chunk(serde_json::Value),
}

/// Spawn `objectiveai-cli instance` and consume its NDJSON stdout as a
/// stream. The endpoint + params + forwarded config ride as an
/// [`InstanceRequest`] over the handshake pipe.
///
/// `stream` — see module doc. `true` follows the instance to EOF;
/// `false` detaches the instance after the `LogStreamReady` handshake.
///
/// Only `functions execute` rides through here — `agents spawn` runs
/// entirely in-process and never goes through the subprocess flow.
pub fn instance_subprocess_stream(
    ctx: &Context,
    endpoint: InstanceEndpoint,
    stream: bool,
) -> Pin<Box<dyn Stream<Item = Result<InstanceItem, Error>> + Send>> {
    let cli_config = ctx.config.clone();
    let fs = ctx.filesystem.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<InstanceItem, Error>>(16);

    tokio::spawn(async move {
        let result = run_subprocess(
            &cli_config,
            fs,
            endpoint,
            stream,
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

async fn run_subprocess(
    cli_config: &crate::run::Config,
    fs: crate::filesystem::Client,
    endpoint: InstanceEndpoint,
    stream: bool,
    tx: tokio::sync::mpsc::Sender<Result<InstanceItem, Error>>,
) -> Result<(), Error> {
    // Resolve every forwarded header / address / auth token using the
    // same env → on-disk-config → SDK-default precedence the regular
    // CLI uses. Owned values, since the parent process drops them as
    // soon as the JSON blob has been written.
    let http = build_http_config(cli_config, &fs).await?;
    let pipes = build_pipe_config(cli_config)?;

    let request = InstanceRequest {
        http,
        pipes,
        endpoint,
    };

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let mut cmd = Command::new(&exe);
    cmd.arg("instance");
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Open the handshake pipe; the read end gets inherited into the
    // child, the write end stays here so we can stream the magic +
    // JSON blob in.
    let (reader, writer) = os_pipe::pipe()
        .map_err(|e| Error::Spawn("handshake pipe".into(), e))?;

    let raw = inheritable_raw(&reader);
    cmd.env(PIPE_ENV, raw);

    // SAFETY: schedule the read end of the pipe to remain in the
    // child after spawn — `pre_exec` (Unix) marks FD_CLOEXEC off;
    // Windows handle inheritance is enabled by std for piped stdio
    // already, but `os_pipe::PipeReader`'s underlying HANDLE has its
    // inheritable bit set by the crate at construction time.
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;
        let fd = reader.as_raw_fd();
        unsafe {
            cmd.pre_exec(move || {
                // Clear FD_CLOEXEC so the fd survives exec(). Leaves
                // other flags untouched.
                let flags = libc::fcntl(fd, libc::F_GETFD);
                if flags < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    // On Windows, detach the child from the parent's console + job
    // object so it survives when the parent exits. Only applies when
    // we plan to release the orphan after the handshake (i.e. when
    // `stream == false`); when streaming, the leaf is following the
    // child to EOF and the inherited console is fine.
    #[cfg(windows)]
    if !stream {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Spawn("objectiveai-cli instance".into(), e))?;
    // Parent no longer needs its copy of the read end — drop it so
    // only the child holds it.
    drop(reader);

    // Hand the JSON blob to the child on a blocking-safe task so the
    // synchronous writer doesn't block the async runtime.
    let request_for_write = request;
    let write_handle = tokio::task::spawn_blocking(move || {
        handshake::write_request(writer, &request_for_write)
    });

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
        let emission: InstanceEmission = match serde_json::from_str(trimmed) {
            Ok(e) => e,
            Err(e) => {
                // Unknown stdout shape. Surface as an Instance error
                // to the consumer instead of panicking — the
                // subprocess may be a different version, or a panic
                // from inside it may have leaked a non-JSON line.
                let _ = tx.send(Err(Error::Instance(format!(
                    "stdout produced a non-InstanceEmission line: {trimmed}; parse error: {e}"
                )))).await;
                break;
            }
        };
        match handle_emission(emission, stream, &tx).await {
            HandleOutcome::DetachReturn => {
                // `stream == false` path: yielded the Id, the
                // instance-runner child keeps running orphaned. Drop
                // the stderr task (the child is on its own now) and
                // return without reaping.
                drop(stderr_task);
                let _ = write_handle.await;
                return Ok(());
            }
            HandleOutcome::SawHandshake => {
                handshake_seen = true;
            }
            HandleOutcome::Continue => {}
            HandleOutcome::ConsumerGone => return Ok(()),
        }
    }

    // Stdout EOF. Reap the child and decide whether the exit was clean.
    let stderr_buf = stderr_task.await.unwrap_or_default();
    let _ = write_handle.await;
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

    if !handshake_seen {
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

#[cfg(unix)]
fn inheritable_raw(reader: &os_pipe::PipeReader) -> String {
    use std::os::fd::AsRawFd;
    reader.as_raw_fd().to_string()
}

#[cfg(windows)]
fn inheritable_raw(reader: &os_pipe::PipeReader) -> String {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{
        HANDLE_FLAG_INHERIT, SetHandleInformation,
    };
    let handle = reader.as_raw_handle();
    // `os_pipe::pipe()` on Windows calls `CreatePipe` with NULL
    // security attributes — the resulting handles are NOT marked
    // inheritable. `std::process::Command::spawn` does set
    // `bInheritHandles=TRUE` on `CreateProcess` (since we pipe
    // stdout/stderr), but unflagged handles still won't transfer.
    // Flip the flag explicitly so the spawn carries this handle
    // into the child.
    unsafe {
        let _ = SetHandleInformation(
            handle as _,
            HANDLE_FLAG_INHERIT,
            HANDLE_FLAG_INHERIT,
        );
    }
    (handle as isize).to_string()
}

enum HandleOutcome {
    /// `stream == false` and we just emitted the Id — the instance
    /// runner is detached, return early without reading more.
    DetachReturn,
    /// `stream == true` and we just emitted the Id — keep reading
    /// chunks until stdout EOF.
    SawHandshake,
    /// Emitted a Chunk or dropped an unrelated Notification — keep
    /// reading.
    Continue,
    /// The receiver was dropped — the consumer abandoned the stream.
    ConsumerGone,
}

async fn handle_emission(
    emission: InstanceEmission,
    stream: bool,
    tx: &tokio::sync::mpsc::Sender<Result<InstanceItem, Error>>,
) -> HandleOutcome {
    match emission {
        InstanceEmission::LogStreamReady { log_stream_ready } => {
            if tx.send(Ok(InstanceItem::Id(log_stream_ready))).await.is_err() {
                return HandleOutcome::ConsumerGone;
            }
            if stream {
                HandleOutcome::SawHandshake
            } else {
                HandleOutcome::DetachReturn
            }
        }
        InstanceEmission::Chunk(value) => {
            if !stream {
                return HandleOutcome::Continue;
            }
            if tx.send(Ok(InstanceItem::Chunk(value))).await.is_err() {
                return HandleOutcome::ConsumerGone;
            }
            HandleOutcome::Continue
        }
        InstanceEmission::Warning { .. } => {
            // Warnings are non-fatal informational lines from the
            // instance runtime. Drop at this boundary — leaves don't
            // expose a warning channel on their typed stream today.
            HandleOutcome::Continue
        }
        InstanceEmission::Error { level: _, fatal: _, message } => {
            // Surface the error to the consumer via the existing
            // `Instance(String)` variant — the cli's local Error
            // enum doesn't currently carry a structured cli::Error
            // payload, so we flatten to its string form. Same
            // ConsumerGone semantics as the other arms on tx failure.
            let text = match &message {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            if tx.send(Err(Error::Instance(text))).await.is_err() {
                return HandleOutcome::ConsumerGone;
            }
            HandleOutcome::Continue
        }
    }
}

/// Resolve every HTTP-client field from cli's `cli_config`, env vars,
/// and on-disk config — mirrors the `build_http_client` precedence chain
/// in [`crate::context`].
async fn build_http_config(
    cli_config: &crate::run::Config,
    fs: &crate::filesystem::Client,
) -> Result<HttpConfig, Error> {
    fn env(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    let mut config = fs.read_config().await.map_err(Error::Filesystem)?;

    let api_address = env("OBJECTIVEAI_ADDRESS").or_else(|| {
        let api = config.api();
        crate::context::compose_url(api.get_address(), api.get_port())
    });

    let objectiveai_authorization = env("OBJECTIVEAI_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_objectiveai_authorization()
            .map(String::from)
    });

    let user_agent =
        env("USER_AGENT").or_else(|| config.api().get_user_agent().map(String::from));

    let x_title =
        env("X_TITLE").or_else(|| config.api().get_x_title().map(String::from));

    let http_referer = env("HTTP_REFERER")
        .or_else(|| config.api().get_http_referer().map(String::from));

    let github_authorization = env("GITHUB_AUTHORIZATION")
        .or_else(|| config.api().get_github_authorization().map(String::from));

    let openrouter_authorization = env("OPENROUTER_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_openrouter_authorization()
            .map(String::from)
    });

    let mcp_authorization: Option<std::collections::HashMap<String, String>> =
        env("MCP_AUTHORIZATION")
            .and_then(|v| serde_json::from_str(&v).ok())
            .or_else(|| {
                config
                    .api()
                    .get_mcp_authorization()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            });

    let viewer_signature = env("VIEWER_SIGNATURE")
        .or_else(|| config.viewer().get_signature().map(String::from));

    let viewer_address = env("VIEWER_ADDRESS").or_else(|| {
        let viewer = config.viewer();
        crate::context::compose_url(viewer.get_address(), viewer.get_port())
    });

    let commit_author_name = env("COMMIT_AUTHOR_NAME")
        .or_else(|| config.api().get_commit_author_name().map(String::from));

    let commit_author_email = env("COMMIT_AUTHOR_EMAIL")
        .or_else(|| config.api().get_commit_author_email().map(String::from));

    Ok(HttpConfig {
        api_address,
        objectiveai_authorization,
        user_agent,
        x_title,
        http_referer,
        github_authorization,
        openrouter_authorization,
        mcp_authorization,
        viewer_signature,
        viewer_address,
        commit_author_name,
        commit_author_email,
        objectiveai_agent_instance_hierarchy: cli_config.agent_instance_hierarchy.clone(),
        mcp_session_id: cli_config.mcp_session_id.clone(),
    })
}

fn build_pipe_config(
    cli_config: &crate::run::Config,
) -> Result<PipeConfig, Error> {
    let config_base_dir = cli_config
        .config_base_dir
        .clone()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_config_base_dir);
    Ok(PipeConfig { config_base_dir })
}

fn default_config_base_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".objectiveai"))
        .unwrap_or_else(|| std::path::PathBuf::from(".objectiveai"))
}
