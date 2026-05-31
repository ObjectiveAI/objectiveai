//! Subprocess bridge to `objectiveai-cli-stream`.
//!
//! Streaming subcommands in this cli no longer talk to the API
//! directly. They spawn `objectiveai-cli-stream` as a per-stream
//! subprocess, forward every HTTP/MCP/agent-id arg as a clap flag,
//! pass the body as `--body <JSON>`, and read the subprocess's
//! NDJSON stdout.
//!
//! Per-chunk responsibilities split as follows:
//!
//! - **cli-stream** owns: opening the WS, sending the conduit,
//!   coalesced log-file writing, emitting `LogStreamReady` once,
//!   per-agent named-pipe listeners.
//! - **cli** owns: arg resolution, per-chunk `inner_errors()` →
//!   `Output::Error(Warn)` emission, in-memory aggregate for the
//!   final summary, final summary `Output::Notification`.
//!
//! Per-chunk `Output::Notification(value=<chunk JSON>)` lines from
//! cli-stream are **consumed silently** by this helper (parsed,
//! inner-errors-emitted, accumulated). `LogStreamReady` lines are
//! forwarded through `handle.emit()`. `Output::Error` lines from
//! cli-stream (e.g. pipe failures) are forwarded as-is.

use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use objectiveai_sdk::cli::output::{
    Error as OutputError, Handle, Level, Notification, NotificationValue, Output, Spawned,
};

/// Spawn `objectiveai-cli-stream <endpoint_path...> --body <JSON>` and
/// consume its NDJSON stdout. Returns the in-memory aggregate built by
/// applying `push` to every chunk Notification the subprocess emits.
///
/// `inner_errors_fn` extracts the per-chunk inner errors (each as a
/// JSON value) so this helper can emit them as `Output::Error(Warn)`
/// via the parent handle. Endpoints with no inner errors (agent
/// completions) pass `|_| Vec::new()`.
pub async fn run<Chunk>(
    cli_config: &crate::Config,
    endpoint_path: &[&str],
    body: &(impl serde::Serialize + ?Sized),
    handle: &Handle,
    inner_errors_fn: impl Fn(&Chunk) -> Vec<serde_json::Value>,
    push: impl Fn(&mut Chunk, &Chunk),
) -> Result<Option<Chunk>, crate::error::Error>
where
    Chunk: DeserializeOwned,
{
    let cli_stream_path = resolve_cli_stream_binary()?;
    let mut cmd = Command::new(&cli_stream_path);

    push_forwarded_args(&mut cmd, cli_config).await?;

    for seg in endpoint_path {
        cmd.arg(seg);
    }

    let body_json = serde_json::to_string(body)
        .expect("body serialization to JSON should not fail for valid params");
    cmd.args(["--body", &body_json]);

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::Error::Spawn("objectiveai-cli-stream".into(), e))?;
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Capture stderr concurrently — forward each line to this
    // process's stderr (same shape as `detach.rs`) and remember the
    // tail for the error path.
    let stderr_task = tokio::spawn(async move {
        let mut buf: Vec<String> = Vec::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            eprintln!("{l}");
            buf.push(l);
        }
        buf
    });

    let mut aggregate: Option<Chunk> = None;
    let mut stdout_lines = BufReader::new(stdout).lines();
    loop {
        let line = match stdout_lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                return Err(crate::error::Error::Spawn(
                    "read cli-stream stdout".into(),
                    e,
                ));
            }
        };
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let out: Output = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("cli-stream stdout produced a non-JSONL line: {trimmed}; parse error: {e}"));
        match out {
            Output::Notification(n) => handle_notification::<Chunk>(
                n,
                handle,
                &inner_errors_fn,
                &push,
                &mut aggregate,
            )
            .await,
            Output::Error(e) => {
                Output::Error(e).emit(handle).await;
            }
        }
    }

    let stderr_buf = stderr_task.await.unwrap_or_default();
    let status = child
        .wait()
        .await
        .map_err(|e| crate::error::Error::Spawn("wait for cli-stream".into(), e))?;

    if !status.success() {
        // If cli-stream surfaced a root chunk error (the aggregate has
        // it), let the caller handle `.error` → `ResponseError`. We
        // return the aggregate so the caller's existing `.error.take()`
        // path still works. If there's no aggregate, the failure was
        // before any chunks arrived — surface as CliStreamSubprocess.
        if aggregate.is_some() {
            return Ok(aggregate);
        }
        let tail: String = stderr_buf
            .iter()
            .rev()
            .take(20)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(crate::error::Error::CliStreamSubprocess {
            code: status.code().unwrap_or(-1),
            stderr_tail: tail,
        });
    }

    Ok(aggregate)
}

async fn handle_notification<Chunk>(
    n: Notification,
    handle: &Handle,
    inner_errors_fn: &impl Fn(&Chunk) -> Vec<serde_json::Value>,
    push: &impl Fn(&mut Chunk, &Chunk),
    aggregate: &mut Option<Chunk>,
) where
    Chunk: DeserializeOwned,
{
    // LogStreamReady arrives as its own typed variant; every chunk
    // emitted by cli-stream lands in the `Other` catch-all (its
    // typed shape varies per endpoint, so cli-stream emits it as a
    // serde_json::Value through the catch-all path).
    match n.value {
        NotificationValue::LogStreamReady(ready) => {
            Output::Notification(Notification {
                agent_id: None,
                value: ready.into(),
            })
            .emit(handle)
            .await;
        }
        NotificationValue::Other(map) => {
            let value = serde_json::Value::Object(map);
            let chunk: Chunk = serde_json::from_value(value).unwrap_or_else(|e| {
                panic!("cli-stream emitted a Notification with unexpected shape: {e}")
            });
            for inner in inner_errors_fn(&chunk) {
                Output::Error(OutputError {
                    level: Level::Warn,
                    fatal: false,
                    message: inner,
                    agent_id: None,
                })
                .emit(handle)
                .await;
            }
            match aggregate.as_mut() {
                Some(a) => push(a, &chunk),
                None => *aggregate = Some(chunk),
            }
        }
        other => {
            panic!("cli-stream emitted an unexpected Notification variant: {other:?}");
        }
    }
}

/// Spawn cli-stream as a detached background process, wait for
/// the [`LogStreamReady`] handshake, emit
/// [`Spawned { agent_id }`](objectiveai_sdk::cli::output::Spawned),
/// and return Ok. The cli-stream child keeps running after this
/// returns; the caller is expected to exit promptly so the orphan
/// can take over the actual completion stream.
///
/// Chunk Notifications and Begin/End are dropped (cli-stream's
/// further output never makes it back to this process). Errors are
/// forwarded via `handle.emit()`. Stderr is mirrored to this
/// process's stderr until the handshake; once we return, the
/// cli-stream child continues writing to its own stderr but nobody
/// reads it.
///
/// On stdout EOF before LogStreamReady, returns
/// `Err(CliStreamSubprocess)` with the stderr tail (same shape as
/// [`run`]).
pub async fn run_detached(
    cli_config: &crate::Config,
    endpoint_path: &[&str],
    body: &(impl serde::Serialize + ?Sized),
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    run_detached_with(cli_config, endpoint_path, body, handle, |ready| {
        Spawned { agent_id: ready }.into()
    })
    .await
}

/// `run_detached` parameterized by the terminal notification emitted
/// once cli-stream's `LogStreamReady` handshake fires. Used by
/// `agents message`'s continuation-fallback path to emit
/// `MessageQueued { agent_id, response_id }` instead of the default
/// `Spawned { agent_id }`.
pub async fn run_detached_with<F>(
    cli_config: &crate::Config,
    endpoint_path: &[&str],
    body: &(impl serde::Serialize + ?Sized),
    handle: &Handle,
    make_notification: F,
) -> Result<(), crate::error::Error>
where
    F: FnOnce(String) -> NotificationValue,
{
    let cli_stream_path = resolve_cli_stream_binary()?;
    let mut cmd = Command::new(&cli_stream_path);

    push_forwarded_args(&mut cmd, cli_config).await?;

    for seg in endpoint_path {
        cmd.arg(seg);
    }

    let body_json = serde_json::to_string(body)
        .expect("body serialization to JSON should not fail for valid params");
    cmd.args(["--body", &body_json]);

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // On Windows, detach the child from the parent's console + job
    // object so it survives when the parent exits.
    //   CREATE_NEW_PROCESS_GROUP (0x00000200): same flag
    //     `api/detach.rs` uses for the parent→child CLI re-exec.
    //   DETACHED_PROCESS (0x00000008): drop the inherited console so
    //     the child isn't taken down with the parent's console
    //     session — required because the cli parent exits as soon
    //     as it sees `LogStreamReady`, while cli-stream keeps
    //     streaming chunks for the rest of the request.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::Error::Spawn("objectiveai-cli-stream".into(), e))?;
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

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
    loop {
        let line = match stdout_lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                return Err(crate::error::Error::Spawn(
                    "read cli-stream stdout".into(),
                    e,
                ));
            }
        };
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let out: Output = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("cli-stream stdout produced a non-JSONL line: {trimmed}; parse error: {e}"));
        match out {
            Output::Error(e) => {
                Output::Error(e).emit(handle).await;
            }
            Output::Notification(n) => {
                // Only LogStreamReady triggers our handshake; every
                // other Notification (chunks) is dropped since the
                // caller does not wait for the completion.
                if let NotificationValue::LogStreamReady(ready) = n.value {
                    Output::Notification(Notification {
                        agent_id: None,
                        value: make_notification(ready.log_stream_ready),
                    })
                    .emit(handle)
                    .await;
                    return Ok(());
                }
            }
        }
    }

    // Stdout EOF before LogStreamReady — child failed early.
    let stderr_buf = stderr_task.await.unwrap_or_default();
    let status = child
        .wait()
        .await
        .map_err(|e| crate::error::Error::Spawn("wait for cli-stream".into(), e))?;
    let tail: String = stderr_buf
        .iter()
        .rev()
        .take(20)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    Err(crate::error::Error::CliStreamSubprocess {
        code: status.code().unwrap_or(-1),
        stderr_tail: tail,
    })
}

/// `<dir-of-current-exe>/objectiveai-cli-stream` (`.exe` on Windows).
/// Same lookup pattern `api/detach.rs` uses via `std::env::current_exe()`.
fn resolve_cli_stream_binary() -> Result<std::path::PathBuf, crate::error::Error> {
    let exe = std::env::current_exe()
        .map_err(|e| crate::error::Error::Spawn("current_exe".into(), e))?;
    let parent = exe.parent().ok_or(crate::error::Error::MissingArgs(
        "current_exe has no parent directory",
    ))?;
    let name = if cfg!(windows) {
        "objectiveai-cli-stream.exe"
    } else {
        "objectiveai-cli-stream"
    };
    Ok(parent.join(name))
}

/// Resolve every cli-stream global flag from cli's `cli_config`,
/// env vars, and on-disk config — mirrors
/// `objectiveai-cli/src/api/client.rs::build_http_client` plus
/// `objectiveai-cli/src/api/conduit.rs::build_handler`'s mcp address
/// resolution.
async fn push_forwarded_args(
    cmd: &mut Command,
    cli_config: &crate::Config,
) -> Result<(), crate::error::Error> {
    fn env(name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    let client = objectiveai_sdk::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        None::<String>,
        None::<String>,
    );
    let mut config = client.read_config().await?;

    let address = env("OBJECTIVEAI_ADDRESS").or_else(|| {
        let api = config.api();
        crate::api::client::compose_url(api.get_address(), api.get_port())
    });
    if let Some(v) = address {
        cmd.args(["--api-address", &v]);
    }

    if let Some(v) = env("OBJECTIVEAI_AUTHORIZATION")
        .or_else(|| config.api().get_objectiveai_authorization().map(String::from))
    {
        cmd.args(["--objectiveai-authorization", &v]);
    }

    if let Some(v) =
        env("USER_AGENT").or_else(|| config.api().get_user_agent().map(String::from))
    {
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

    if let Some(v) = env("OPENROUTER_AUTHORIZATION")
        .or_else(|| config.api().get_openrouter_authorization().map(String::from))
    {
        cmd.args(["--openrouter-authorization", &v]);
    }

    // MCP authorization in cli-stream is `--mcp-authorization <JSON>`
    // (HashMap<String,String>). Env carries pre-encoded JSON; config
    // carries an in-memory map that needs re-encoding.
    let mcp_auth_json = env("MCP_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_mcp_authorization()
            .map(|m| serde_json::to_string(m).expect("encoding String→String map"))
    });
    if let Some(v) = mcp_auth_json {
        cmd.args(["--mcp-authorization", &v]);
    }

    if let Some(v) = env("VIEWER_SIGNATURE")
        .or_else(|| config.viewer().get_signature().map(String::from))
    {
        cmd.args(["--viewer-signature", &v]);
    }

    let viewer_address = env("VIEWER_ADDRESS").or_else(|| {
        let viewer = config.viewer();
        crate::api::client::compose_url(viewer.get_address(), viewer.get_port())
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

    cmd.args(["--objectiveai-agent-id", &cli_config.agent_id]);

    if let Some(ref v) = cli_config.mcp_session_id {
        cmd.args(["--mcp-session-id", v]);
    }

    // config_base_dir is required for cli-stream's pipes + log layout.
    if let Some(ref base) = cli_config.config_base_dir {
        cmd.args(["--config-base-dir", base]);
    }

    // mcp_address resolution — same shape as the deleted
    // `api/conduit.rs::build_handler`.
    let mcp_url = env("OBJECTIVEAI_MCP_ADDRESS").or_else(|| {
        let mcp = config.mcp();
        let port = env("OBJECTIVEAI_MCP_PORT")
            .and_then(|s| s.parse::<u16>().ok())
            .or_else(|| mcp.get_port());
        crate::api::client::compose_url(mcp.get_address(), port)
    });
    if let Some(v) = mcp_url {
        cmd.args(["--mcp-address", &v]);
    }

    Ok(())
}
