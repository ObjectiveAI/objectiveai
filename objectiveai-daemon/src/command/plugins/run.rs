//! `plugins run` — bare-naked port of legacy `dispatch_external`.
//!
//! Resolves the installed plugin's exec command (the tools model:
//! the manifest's per-OS argv plus the caller's args, run with CWD =
//! the plugin's `cli/` folder), spawns it with piped
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

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::plugins::run::{Request, ResponseItem};
use objectiveai_sdk::cli::plugins::Output as PluginOutput;
use objectiveai_sdk::cli::{Error as CliError, ErrorType as CliErrorType};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::child_io::{PipeEvent, spawn_stdout_reader};
use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let coord = format!("{}/{}/{}", request.owner, request.name, request.version);
    let (exec, cli_dir) = ctx
        .filesystem
        .resolve_plugin(&request.owner, &request.name, &request.version)
        .await
        .ok_or_else(|| Error::PluginNotFound(coord.clone()))?;

    // The command is the plugin's exec vector merged with the
    // caller's args, verbatim. The first element is the program; the
    // rest are its arguments. CWD is the plugin's `cli/` folder —
    // always — with the same relative-path program resolution tools
    // use.
    let mut argv = exec;
    argv.extend(request.args.iter().cloned());
    let mut argv = argv.into_iter();
    let program = argv
        .next()
        .ok_or_else(|| Error::PluginNotFound(format!("{coord} (empty exec)")))?;
    let program = crate::spawn::resolve_program(program, &cli_dir);

    // Per-plugin scratch space inside the (transient) state tree —
    // plugins that persist files write here, never into their own
    // (possibly committed) install folder.
    let state_dir = ctx
        .filesystem
        .state_dir()
        .join("plugins")
        .join(&request.owner)
        .join(&request.name)
        .join(&request.version);
    tokio::fs::create_dir_all(&state_dir)
        .await
        .map_err(Error::PluginSpawn)?;

    // Per-plugin database compartment: an owned schema plus readonly
    // access to the base objectiveai tables, handed to the child as
    // a role-scoped connection URL. Provisioning is idempotent; a
    // failure fails the run loudly rather than spawning a child with
    // a silently missing database.
    let postgres_url = crate::db::compartment::ensure(
        ctx.db_handle().await?,
        crate::db::compartment::Kind::Plugin,
        &request.owner,
        &request.name,
        &request.version,
    )
    .await?;

    // Context for nested (plugin-originated) commands: this caller's
    // ctx verbatim. Plugins carry no special routing identity — their
    // nested commands broadcast on /listen like any other run, and
    // plugins observe the daemon with the SAME WebSocket executor /
    // listeners every other client uses.
    let nested_ctx = ctx.clone();

    let mut cmd = Command::new(&program);
    cmd.args(argv)
        .current_dir(&cli_dir)
        .env("OBJECTIVEAI_STATE_DIR", &state_dir)
        .env("OBJECTIVEAI_BIN_DIR", &cli_dir)
        .env("OBJECTIVEAI_POSTGRES_URL", postgres_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::spawn::apply_config_env(&mut cmd, &nested_ctx.config);

    // Leash the plugin to the cli process: a plugin `mcp begin` is a
    // long-lived MCP server, and the conduit holds its Child inside a
    // detached drain task — so without an OS-level leash it outlives the
    // cli. `subprocess_reaper::spawn` sets `kill_on_drop` AND the OS leash
    // (job object / PR_SET_PDEATHSIG / kqueue guardian) so it dies with
    // the cli even on a force-kill.
    let mut child =
        objectiveai_sdk::subprocess_reaper::spawn(&mut cmd).map_err(Error::PluginSpawn)?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stdin = child.stdin.take().expect("stdin was piped");
    let plugin_stdin: Arc<Mutex<ChildStdin>> = Arc::new(Mutex::new(stdin));

    // Capture the plugin's stderr (stdout is the protocol channel) into
    // `objectiveai.plugin_messages` on a dedicated task that OWNS the
    // pipe and reads it line-by-line — the OS pipe is the buffer, so a
    // slow DB just backpressures the plugin's stderr writes. Joined at
    // the end of the stream so every line is flushed before `plugins
    // run` completes.
    let stderr_writer = spawn_stderr_log_writer(
        ctx.db_client().await?.clone(),
        request.owner.clone(),
        request.name.clone(),
        request.version.clone(),
        ctx.config.agent_instance_hierarchy.clone(),
        ctx.config.response_id.clone(),
        stderr,
    );

    let mut events = spawn_stdout_reader(stdout);

    let stream = async_stream::stream! {
        let mut command_tasks: Vec<(Option<String>, JoinHandle<i32>)> = Vec::new();
        while let Some(event) = events.recv().await {
            match event {
                PipeEvent::Stderr(_) => {
                    // stderr is owned by the DB log-writer task, not this
                    // reader (stdout-only), so this never fires — kept for
                    // match exhaustiveness over `PipeEvent`.
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
                            let task = run_nested_command(
                                nested_ctx.clone(),
                                c.command,
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

        // Await any in-flight Command tasks the plugin queued before its
        // stdout EOF. Each task writes its OWN terminal `CommandComplete`
        // when its stream ends (see `run_nested_command`), so here we just
        // let them finish before dropping plugin stdin.
        for (_id, task) in command_tasks {
            let _ = task.await;
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

        // The child has exited, so its stderr pipe is closed; join the
        // log-writer task so every captured line is committed before the
        // run stream ends. Surface the first DB/read error if any.
        match stderr_writer.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                yield Err(e);
            }
            Err(_join) => {}
        }
    };

    Ok(Box::pin(stream))
}

/// Spawn the task that owns the plugin's stderr pipe and appends every
/// line to `objectiveai.plugin_messages`. Reads one line at a time, so
/// the OS pipe backpressures a slow DB rather than buffering unbounded.
/// The returned handle is awaited before the run stream completes; the
/// first insert/read error is surfaced through it.
fn spawn_stderr_log_writer(
    pool: crate::db::Pool,
    owner: String,
    name: String,
    version: String,
    agent_instance_hierarchy: String,
    response_id: Option<String>,
    stderr: tokio::process::ChildStderr,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let created_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    crate::db::logs::insert_plugin_message(
                        &pool,
                        &owner,
                        &name,
                        &version,
                        &agent_instance_hierarchy,
                        response_id.as_deref(),
                        created_at,
                        &line,
                    )
                    .await
                    .map_err(Error::from)?;
                }
                Ok(None) => return Ok(()),
                Err(e) => return Err(Error::PluginRead(e)),
            }
        }
    })
}

/// Dispatch a plugin-originated command IN-PROCESS — no subprocess, no
/// Postgres re-bootstrap. `command` arrives as an already-tokenized
/// argv vector (the plugin executor carries it structured), so it runs
/// through the very same `crate::run` entry the cli binary uses without
/// any re-tokenization — an argument value containing whitespace stays a
/// single token. Dispatched against `ctx` (which already carries this
/// caller's identity plus the plugin coordinate). The body is a mirror
/// of `main.rs::run_command`: every line that binary would write to
/// stdout is instead forwarded into `plugin_stdin` wrapped in a
/// [`PluginCommandResponse`]. Returns an exit code for the terminal
/// `CommandComplete` (the tool's code on a `ToolExit`, else 0/1).
fn run_nested_command(
    ctx: Context,
    command: Vec<String>,
    plugin_stdin: Arc<Mutex<ChildStdin>>,
    id: Option<String>,
) -> JoinHandle<i32> {
    tokio::spawn(async move {
        let id = id.as_deref();
        let exit_code = run_nested_inner(ctx, command, plugin_stdin.clone(), id).await;
        // Per-command stream terminator: tell the plugin THIS command's
        // response stream is exhausted (carries the id) so its executor
        // ends the right stream. Written promptly when the stream ends —
        // not deferred to plugin-stdout EOF — and swallowed by the executor.
        let _ = write_envelope(
            &plugin_stdin,
            &PluginCommandResponse {
                id,
                value: CommandComplete {
                    kind: "command_complete",
                    exit_code,
                },
            },
        )
        .await;
        exit_code
    })
}

/// Run one plugin-originated command in-process, forwarding each output
/// line into `plugin_stdin` as a [`PluginCommandResponse`]. Returns the
/// exit code (the tool's code on a `ToolExit`, else 0/1); the caller
/// ([`run_nested_command`]) writes the terminal [`CommandComplete`] once
/// this returns.
async fn run_nested_inner(
    ctx: Context,
    command: Vec<String>,
    plugin_stdin: Arc<Mutex<ChildStdin>>,
    id: Option<&str>,
) -> i32 {
    // Argv is already tokenized by the plugin executor — one
    // element per argument. No `split_whitespace`, so a value like
    // `--simple "a b c"` is not re-split into separate tokens.
    let tokens: Vec<String> = command;

    // A plugin may not invoke `plugins` or `tools` commands — no
    // running another plugin, no running a tool. Forward the same
    // error line the cli would emit for the forbidden cases.
    let forbidden = match tokens.first().map(String::as_str) {
        Some("plugins") => Some("plugins"),
        Some("tools") => Some("tools"),
        _ => None,
    };
    if let Some(kind) = forbidden {
        let _ = forward_error(
            &plugin_stdin,
            id,
            &Error::PluginCommandForbidden(kind),
            Some(true),
        )
        .await;
        return 1;
    }

    // `args[0]` is the program name, which `crate::run` strips
    // unconditionally; the plugin's command is just the subcommand
    // tokens, so prepend a placeholder.
    let mut args: Vec<String> = vec!["objectiveai".to_string()];
    args.extend(tokens);

    // A mirror of `main.rs::run_command`: drive the same `crate::run`
    // stream, but forward each line into the plugin's stdin (wrapped
    // in a `PluginCommandResponse`) instead of writing it to stdout.
    let run_stream = match crate::run(args, Some(ctx)).await {
        Ok(s) => s,
        Err(e) => {
            if let Error::ClapParse(ref clap_err) = e {
                if crate::is_informational(clap_err) {
                    let _ = forward_help(&plugin_stdin, id, &clap_err.to_string()).await;
                    return 0;
                }
            }
            let _ = forward_error(&plugin_stdin, id, &e, Some(true)).await;
            return match e {
                Error::ToolExit(code) => code,
                _ => 1,
            };
        }
    };
    // Both arms forward each item to the plugin's stdin as a line;
    // `drain` is generic over the item type (typed root items vs
    // post-transform JSON).
    let last_tool_exit = match run_stream {
        crate::RunStream::Execute(stream) => drain(&plugin_stdin, id, stream).await,
        crate::RunStream::ExecuteTransform(stream) => drain(&plugin_stdin, id, stream).await,
    };
    last_tool_exit.unwrap_or(0)
}

/// Drain a run stream into the plugin's stdin — one
/// [`PluginCommandResponse`] line per item. Returns the last `ToolExit`
/// code seen (surfaced as an `Err` item). Generic over the item type so
/// it serves both `RunStream` arms. Stops early if the plugin's stdin
/// closes.
async fn drain<S, T>(
    plugin_stdin: &Arc<Mutex<ChildStdin>>,
    id: Option<&str>,
    mut stream: S,
) -> Option<i32>
where
    S: Stream<Item = Result<T, Error>> + Unpin,
    T: Serialize,
{
    let mut last_tool_exit: Option<i32> = None;
    while let Some(item) = stream.next().await {
        let written = match item {
            Ok(value) => forward_line(plugin_stdin, id, &value).await,
            Err(e) => {
                if let Error::ToolExit(code) = &e {
                    last_tool_exit = Some(*code);
                }
                forward_error(plugin_stdin, id, &e, None).await
            }
        };
        if written.is_err() {
            // Plugin's stdin is gone; abandon the run.
            break;
        }
    }
    last_tool_exit
}

/// Mirror of `main.rs::write_line`: serialize `value` and forward it to
/// the plugin's stdin in a [`PluginCommandResponse`] envelope.
async fn forward_line<T: Serialize>(
    plugin_stdin: &Arc<Mutex<ChildStdin>>,
    id: Option<&str>,
    value: &T,
) -> std::io::Result<()> {
    write_envelope(plugin_stdin, &PluginCommandResponse { id, value }).await
}

/// Mirror of `main.rs::write_error_line`.
async fn forward_error(
    plugin_stdin: &Arc<Mutex<ChildStdin>>,
    id: Option<&str>,
    e: &Error,
    fatal: Option<bool>,
) -> std::io::Result<()> {
    let payload = CliError {
        r#type: CliErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal,
        message: e.output_message(),
    };
    forward_line(plugin_stdin, id, &payload).await
}

/// Mirror of `main.rs::write_help_line`.
async fn forward_help(
    plugin_stdin: &Arc<Mutex<ChildStdin>>,
    id: Option<&str>,
    help: &str,
) -> std::io::Result<()> {
    let payload = serde_json::json!({ "type": "help", "help": help });
    forward_line(plugin_stdin, id, &payload).await
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
struct PluginCommandResponse<'a, T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    value: T,
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
