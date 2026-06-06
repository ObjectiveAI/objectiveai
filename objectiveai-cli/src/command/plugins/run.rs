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
use objectiveai_sdk::cli::command::plugins::run::{Request, ResponseItem};
use objectiveai_sdk::cli::plugins::Output as PluginOutput;
use objectiveai_sdk::cli::{Error as CliError, ErrorType as CliErrorType};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::child_io::{PipeEvent, spawn_pipe_reader};
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

    let mut events = spawn_pipe_reader(stdout, stderr);
    let cli_config = ctx.config.clone();

    let stream = async_stream::stream! {
        let mut command_tasks: Vec<(Option<String>, JoinHandle<i32>)> = Vec::new();
        while let Some(event) = events.recv().await {
            match event {
                PipeEvent::Stderr(_) => {
                    // Bare anonymous error — no level, no fatal, no
                    // message. Stops at "something went wrong on
                    // stderr" by deliberate host policy.
                    yield Ok(ResponseItem::Error(CliError {
                        r#type: CliErrorType::Error,
                        level: None,
                        fatal: None,
                        message: serde_json::Value::Null,
                    }));
                }
                PipeEvent::Stdout(trimmed) => {
                    match serde_json::from_str::<PluginOutput>(&trimmed) {
                        Ok(PluginOutput::Error(e)) => {
                            yield Ok(ResponseItem::Error(e));
                        }
                        Ok(PluginOutput::Mcp(mcp)) => {
                            yield Ok(ResponseItem::Mcp(mcp));
                        }
                        Ok(PluginOutput::Command(c)) => {
                            // Command requests are host-internal —
                            // the CLI intercepts them to drive the
                            // bidirectional protocol back into the
                            // plugin's stdin and does NOT surface
                            // them on the user-visible `ResponseItem`
                            // stream.
                            let task_id = Some(c.id);
                            let task = spawn_nested_command(
                                c.command,
                                cli_config.clone(),
                                plugin_stdin.clone(),
                                task_id.clone(),
                            );
                            command_tasks.push((task_id, task));
                        }
                        Ok(PluginOutput::Notification(value)) => {
                            yield Ok(ResponseItem::Notification(value));
                        }
                        Err(_) => {
                            // Legacy fallback: surface the raw line
                            // as a notification so unparseable plugin
                            // output is at least observable rather
                            // than silently dropped.
                            yield Ok(ResponseItem::Notification(
                                serde_json::Value::String(trimmed),
                            ));
                        }
                    }
                }
                PipeEvent::StdoutEof | PipeEvent::StderrEof => {}
                PipeEvent::StdoutErr(e) | PipeEvent::StderrErr(e) => {
                    yield Err(Error::PluginRead(e));
                    return;
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
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::run as sdk;
    use objectiveai_sdk::cli::command::plugins::run::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
