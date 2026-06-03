//! `plugins run` — bare-naked port of legacy `dispatch_external`.
//!
//! Resolves the installed plugin binary, spawns it with piped
//! stdin/stdout/stderr, and yields each parsed line from the plugin's
//! stdout as a [`ResponseItem`] as it arrives. The bidirectional
//! protocol — plugin emits a `Command` request, the host runs it and
//! streams the result back into the plugin's stdin wrapped in a
//! `PluginCommandResponse` envelope, terminated by a
//! `CommandComplete` marker — stays internal to the leaf. Consumers
//! observe Command requests as stream items but the actual execution
//! and stdin write-back happens here.

use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;

use futures::Stream;
use objectiveai_sdk::cli::command::plugins::run::{
    Request, ResponseItem, ResponseTyped,
};
use objectiveai_sdk::cli::plugins::{Output as PluginOutput, TypedOutput as TypedPluginOutput};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let exe = ctx
        .filesystem
        .resolve_plugin(&request.name)
        .await
        .ok_or_else(|| Error::PluginNotFound(request.name.clone()))?;

    let mut cmd = Command::new(&exe);
    cmd.args(&request.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::spawn::apply_config_env(&mut cmd, &ctx.config);

    let mut child = cmd.spawn().map_err(Error::PluginSpawn)?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stdin = child.stdin.take().expect("stdin was piped");
    let plugin_stdin: Arc<Mutex<ChildStdin>> = Arc::new(Mutex::new(stdin));

    // Stderr → host's stderr, best-effort, no backpressure into the stream.
    let stderr_task = tokio::spawn(forward_stderr(stderr));

    let cli_config = ctx.config.clone();

    let stream = async_stream::stream! {
        let mut command_tasks: Vec<(Option<String>, JoinHandle<i32>)> = Vec::new();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            let n = match reader.read_line(&mut line).await {
                Ok(n) => n,
                Err(e) => {
                    yield Err(Error::PluginRead(e));
                    return;
                }
            };
            if n == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            match serde_json::from_str::<PluginOutput>(trimmed) {
                Ok(PluginOutput::Typed(TypedPluginOutput::Error(e))) => {
                    yield Ok(ResponseItem::Error(e));
                }
                Ok(PluginOutput::Typed(TypedPluginOutput::Mcp(mcp))) => {
                    // `ResponseTyped::Mcp` only carries `url`; headers
                    // are dropped on the bare-naked wire. Separate SDK
                    // widening if a plugin needs them.
                    yield Ok(ResponseItem::Typed(ResponseTyped::Mcp { url: mcp.url }));
                }
                Ok(PluginOutput::Typed(TypedPluginOutput::Command { id, command })) => {
                    // Yield the request for observability, then spawn
                    // the writer task that drives the bidirectional
                    // protocol back to the plugin's stdin.
                    let observe_id = id.clone();
                    let observe_cmd = command.clone();
                    let task_id = Some(id);
                    let task = spawn_nested_command(
                        command,
                        cli_config.clone(),
                        plugin_stdin.clone(),
                        task_id.clone(),
                    );
                    command_tasks.push((task_id, task));
                    yield Ok(ResponseItem::Typed(ResponseTyped::Command {
                        id: Some(observe_id),
                        command: observe_cmd,
                    }));
                }
                Ok(PluginOutput::Notification(value)) => {
                    yield Ok(ResponseItem::Notification(value));
                }
                Err(_) => {
                    // Legacy fallback: surface the raw line as a
                    // notification so unparseable plugin output is at
                    // least observable rather than silently dropped.
                    yield Ok(ResponseItem::Notification(
                        serde_json::Value::String(trimmed.to_string()),
                    ));
                }
            }
        }

        // Drain any in-flight Command tasks the plugin queued before
        // its stdout EOF. Each task gets a terminal `CommandComplete`
        // written to plugin stdin so the plugin sees the run boundary
        // even when it didn't mint a correlation id.
        for (id, task) in command_tasks {
            let exit_code = task.await.unwrap_or(-1);
            let envelope = PluginCommandResponse {
                id: id.as_deref(),
                value: serde_json::to_value(CommandComplete {
                    kind: "command_complete",
                    exit_code,
                })
                .expect("CommandComplete serializes"),
            };
            let _ = write_envelope(&plugin_stdin, &envelope).await;
        }

        // Drop our reference to plugin stdin so the kernel pipe closes
        // and a polite plugin sees EOF on its stdin read.
        drop(plugin_stdin);
        let _ = stderr_task.await;

        match child.wait().await {
            Ok(status) if status.success() => {}
            Ok(status) => {
                yield Err(Error::PluginExit(status.code().unwrap_or(1)));
            }
            Err(e) => {
                yield Err(Error::PluginRead(e));
            }
        }
    };

    Ok(Box::pin(stream))
}

/// Whitespace-tokenize `command`, spawn `objectiveai-cli` (the
/// current binary) with those argv, capture stdout line-by-line, and
/// write each parsed line back to `plugin_stdin` wrapped in a
/// [`PluginCommandResponse`] envelope. Returns the nested process's
/// exit code (used by the outer drain to stamp the terminal
/// `CommandComplete`).
///
/// Tokenization matches legacy: whitespace-only, no shlex.
fn spawn_nested_command(
    command: String,
    cli_config: crate::run::Config,
    plugin_stdin: Arc<Mutex<ChildStdin>>,
    id: Option<String>,
) -> JoinHandle<i32> {
    tokio::spawn(async move {
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return -1,
        };
        let tokens: Vec<String> =
            command.split_whitespace().map(String::from).collect();
        let mut cmd = Command::new(&exe);
        cmd.args(&tokens)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        crate::spawn::apply_config_env(&mut cmd, &cli_config);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return -1,
        };
        let stdout = child.stdout.take().expect("stdout was piped");
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(raw)) = lines.next_line().await {
            let trimmed = raw.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::Value::String(trimmed.to_string()));
            let envelope = PluginCommandResponse {
                id: id.as_deref(),
                value,
            };
            if write_envelope(&plugin_stdin, &envelope).await.is_err() {
                // Plugin's stdin is gone — abandon the run.
                break;
            }
        }
        match child.wait().await {
            Ok(s) => s.code().unwrap_or(-1),
            Err(_) => -1,
        }
    })
}

async fn write_envelope<T: Serialize>(
    stdin: &Arc<Mutex<ChildStdin>>,
    envelope: &T,
) -> std::io::Result<()> {
    let line = serde_json::to_string(envelope).expect("envelope serializes");
    let mut guard = stdin.lock().await;
    guard.write_all(line.as_bytes()).await?;
    guard.write_all(b"\n").await?;
    guard.flush().await?;
    Ok(())
}

async fn forward_stderr<R>(stream: R)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                eprintln!("{trimmed}");
            }
        }
    }
}

/// Wire envelope for nested-command output streamed back to plugin
/// stdin. Matches `cli.plugins.PluginCommandResponse.json`. Defined
/// locally rather than in the SDK because the SDK's `cli/output`
/// module that hosts the canonical type is currently torn-up.
#[derive(Serialize)]
struct PluginCommandResponse<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    value: serde_json::Value,
}

/// Terminal marker written to plugin stdin after each nested command
/// finishes. Matches `cli.output.notification.CommandComplete.json`.
#[derive(Serialize)]
struct CommandComplete {
    #[serde(rename = "type")]
    kind: &'static str,
    exit_code: i32,
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::run as sdk;
    use objectiveai_sdk::cli::command::plugins::run::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::run as sdk;
    use objectiveai_sdk::cli::command::plugins::run::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::ResponseItem))
    }
}
