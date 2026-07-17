//! The user-facing `objectiveai` CLI — a thin HTTP client.
//!
//! Every command is a typed `cli::command::Request`. This binary does
//! NOT run any command logic: it clap-parses argv into a `Request`
//! locally, ensures the resident `objectiveai-daemon` is up, and ships
//! the `Request` to the daemon's `/execute` route (POST) via the SDK
//! [`SseCommandExecutor`]. The daemon runs it IN-PROCESS and streams the
//! result back as SSE — one JSON line per item, exactly the stdout JSONL
//! shapes the daemon itself would have written. We drain those lines to
//! stdout verbatim.
//!
//! Parsing happens BEFORE any daemon contact, so `--help` / `--version`
//! / parse errors never spawn or dial a daemon.
//!
//! **Which daemon?** A three-rung ladder (see [`connect`]):
//! 1. `DAEMON_ADDRESS` env — an explicit per-invocation override,
//!    presented with the `DAEMON_SIGNATURE` env verbatim.
//! 2. The state config's `daemon` section, when its `address` is set:
//!    dial THAT daemon, presenting the section's stored `signature`
//!    (the credential for a daemon whose secret we don't hold).
//! 3. Otherwise the LOCAL daemon: a `spawn_until_published` against
//!    the `objectiveai-daemon` binary, keyed on the per-state
//!    `plugins-daemon` lock, whose published contents are the daemon's
//!    connect `http://` URL (mirrors
//!    `objectiveai_daemon::command::daemon::spawn`). An address-less
//!    `daemon` section describes THIS local daemon — its stored
//!    `signature` is IGNORED and the presented signature is DERIVED
//!    from the section's `secret` (the same math the daemon verifies
//!    with; trusting the stored signature would make the secret
//!    pointless).

use std::path::PathBuf;

use futures::StreamExt;
use objectiveai_sdk::cli::command::command_executor::sse;
use objectiveai_sdk::cli::command::{
    AgentArguments, CommandExecutor, ParseError, Request, SseCommandExecutor, parse_request,
};
use tokio::io::AsyncWriteExt;

/// The resident daemon's per-state singleton lock key. Must match
/// `objectiveai_daemon::command::daemon::DAEMON_LOCK_KEY` — the daemon
/// (acquire) and this client (spawn-until-published) key off the same
/// name. Hardcoded rather than shared because the thin client cannot
/// depend on the heavy daemon crate (same pattern as the viewer's
/// hardcoded `"viewer"` key).
const DAEMON_LOCK_KEY: &str = "plugins-daemon";

#[tokio::main]
async fn main() {
    // Two-tier dotenv, matching objectiveai-daemon/src/main.rs: the CWD
    // `.env` overrides `<OBJECTIVEAI_DIR>/.env`. dotenv never overrides
    // an already-set var, so loading the CWD file FIRST makes it win,
    // and the real environment still wins over both.
    let _ = dotenv::dotenv();
    let dir = objectiveai_dir();
    let _ = dotenv::from_path(dir.join(".env"));

    let args: Vec<String> = std::env::args().collect();
    let code = run(args).await;
    std::process::exit(code);
}

/// Layout root (`OBJECTIVEAI_DIR`); default `~/.objectiveai`. Same
/// resolution as the daemon's `filesystem::Client`.
fn objectiveai_dir() -> PathBuf {
    std::env::var_os("OBJECTIVEAI_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".objectiveai")
        })
}

async fn run(args: Vec<String>) -> i32 {
    let mut stdout = tokio::io::stdout();

    // Parse LOCALLY: `--help` / `--version` / bad args resolve here and
    // never touch a daemon. `parse_request` prepends its own canonical
    // bin name, so drop argv[0].
    let request = match parse_request(args.get(1..).unwrap_or_default()) {
        Ok(request) => request,
        Err(ParseError::Clap(clap_err)) => {
            // `--help` / `--version` / no-subcommand → informational,
            // rendered as a `help` line with exit 0 so pipelines aren't
            // penalised (mirrors objectiveai-daemon/src/main.rs).
            if is_informational(&clap_err) {
                write_help_line(&mut stdout, &clap_err.to_string()).await;
                return 0;
            }
            write_error_line(&mut stdout, clap_err.to_string(), Some(true)).await;
            return 1;
        }
        Err(ParseError::FromArgs(e)) => {
            write_error_line(&mut stdout, e.to_string(), Some(true)).await;
            return 1;
        }
    };

    // `daemon kill` would make the daemon kill ITSELF over `/execute`
    // (a self-`TerminateProcess` truncates the response), so it is
    // handled client-side. With every server a leashed daemon child,
    // this is also the whole-teardown command: the OS leash takes db /
    // api / mcp / viewer / laboratory host down with the daemon.
    if let Request::Daemon(objectiveai_sdk::cli::command::daemon::Request::Kill(_)) = &request {
        return handle_daemon_kill(&mut stdout).await;
    }

    // Ensure the daemon is up and build the HTTP executor + identity bag.
    let (executor, agent_arguments) = match connect().await {
        Ok(pair) => pair,
        Err(message) => {
            write_error_line(&mut stdout, message, Some(true)).await;
            return 1;
        }
    };

    // Ship the whole request; drain each JSON line to stdout verbatim.
    // `serde_json::Value` decodes any item shape (typed root items and
    // post-transform JSON alike), and the executor surfaces the daemon's
    // structured error lines as `Error::Cli`.
    match executor
        .execute::<_, serde_json::Value>(request, Some(&agent_arguments))
        .await
    {
        Ok(mut stream) => {
            let mut saw_error = false;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(value) => write_json_line(&mut stdout, &value).await,
                    Err(e) => {
                        saw_error = true;
                        write_execute_error(&mut stdout, e).await;
                    }
                }
            }
            // Tool exit codes don't cross the `/execute` wire (the
            // structured error line carries no code — the same contract
            // the viewer runs under), so any error line maps to 1.
            if saw_error { 1 } else { 0 }
        }
        Err(e) => {
            write_execute_error(&mut stdout, e).await;
            1
        }
    }
}

/// On-disk layout the CLI needs for the bootstrap + the client-side kills.
struct Layout {
    dir: PathBuf,
    state: String,
    lock_dir: PathBuf,
    daemon_exe: PathBuf,
}

/// Resolve the layout from the environment (same defaults as the daemon's
/// `filesystem::Client`).
fn resolve_layout() -> Layout {
    let dir = objectiveai_dir();
    let state = std::env::var("OBJECTIVEAI_STATE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string());
    let lock_dir = dir.join("state").join(&state).join("locks");
    let daemon_bin = if cfg!(windows) {
        "objectiveai-daemon.exe"
    } else {
        "objectiveai-daemon"
    };
    let daemon_exe = dir.join("bin").join(daemon_bin);
    Layout { dir, state, lock_dir, daemon_exe }
}

/// The daemon auth signature from the environment
/// (`DAEMON_SIGNATURE`, verbatim `sha256=<hex(SHA256(secret))>`) —
/// the fallback credential when no on-disk `daemon` section decides.
/// `DAEMON_SECRET` is only for handing a spawned daemon its `SECRET`.
/// `None` = connect unauthenticated (the daemon must be open).
fn daemon_signature_env() -> Option<String> {
    std::env::var("DAEMON_SIGNATURE").ok().filter(|s| !s.is_empty())
}

/// `sha256=<hex(SHA256(secret))>` — the client-side half of the
/// daemon's verify math (mirrors
/// `objectiveai_daemon::http::daemon_auth::derive_signature`; hardcoded
/// because the thin client cannot depend on the heavy daemon crate).
fn derive_signature(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256={}", hex::encode(Sha256::digest(secret.as_bytes())))
}

/// The `daemon` section of the state's on-disk `config.json` — the
/// CLI-relevant sliver, a field-for-field mirror of the daemon's
/// `filesystem::config::DaemonConfig` (hardcoded, same no-daemon-dep
/// pattern as [`DAEMON_LOCK_KEY`]). `address: Some` names the daemon
/// the CLI should dial (with the stored `signature` as its
/// credential); `address: None` describes the LOCAL daemon (its
/// `secret` is what auth verifies against — the stored `signature` is
/// ignored and the credential is derived from the secret instead).
#[derive(serde::Deserialize)]
struct DaemonConfigSection {
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    secret: Option<String>,
    #[serde(default)]
    signature: Option<String>,
}

/// The state config, reduced to what the CLI reads.
#[derive(serde::Deserialize)]
struct StateConfig {
    #[serde(default)]
    daemon: Option<DaemonConfigSection>,
}

/// Read the state's `config.json` `daemon` section. A missing file or
/// section is `None` (the env/local ladder decides); a file that
/// exists but cannot be read or parsed is a LOUD error — silently
/// falling back could dial the wrong daemon.
async fn read_daemon_config(layout: &Layout) -> Result<Option<DaemonConfigSection>, String> {
    let path = layout
        .dir
        .join("state")
        .join(&layout.state)
        .join("config.json");
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    serde_json::from_slice::<StateConfig>(&bytes)
        .map(|config| config.daemon)
        .map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Build a `/execute` [`SseCommandExecutor`] for an already-known daemon
/// `http://` URL (no spawn), presenting `signature` when given.
fn executor_for(url: &str, signature: Option<String>) -> SseCommandExecutor {
    let executor = SseCommandExecutor::new(format!("{url}/execute"));
    match signature {
        Some(signature) => executor.signature(signature),
        None => executor,
    }
}

/// Ensure the right daemon is reachable and return a `/execute`
/// [`SseCommandExecutor`] plus the per-request identity override to
/// send with every command. The selection ladder is documented on the
/// module.
async fn connect() -> Result<(SseCommandExecutor, AgentArguments), String> {
    // Rung 1 — env override: when `DAEMON_ADDRESS` is set, connect to
    // that daemon directly and NEVER spawn a local one; an explicit
    // per-invocation choice outranks the persisted config.
    if let Ok(addr) = std::env::var("DAEMON_ADDRESS")
        && !addr.is_empty()
    {
        return Ok((
            executor_for(&addr, daemon_signature_env()),
            agent_arguments_from_env(),
        ));
    }

    let layout = resolve_layout();

    // Rung 2 — the persisted `daemon` section, when it names an
    // address: dial THAT daemon with its stored signature (we don't
    // hold a remote daemon's secret; the signature IS the credential).
    let section = read_daemon_config(&layout).await?;
    if let Some(section) = &section
        && let Some(address) = section.address.as_deref().filter(|s| !s.is_empty())
    {
        let signature = section.signature.clone().filter(|s| !s.is_empty());
        return Ok((executor_for(address, signature), agent_arguments_from_env()));
    }

    // Rung 3 — the local daemon. Idempotent: returns immediately if
    // the daemon already holds its lock; otherwise launches it once
    // (as its own foreground process) and waits for readiness. The
    // published lock content is the daemon's connect `http://` URL.
    // Mirrors `objectiveai_daemon::command::daemon::spawn::spawn`, but
    // launches the `objectiveai-daemon` binary rather than re-execing
    // this one.
    let url = objectiveai_sdk::lockfile::spawn_until_published(
        &layout.daemon_exe,
        &layout.lock_dir,
        DAEMON_LOCK_KEY,
        |cmd| {
            cmd.arg("daemon")
                .arg("spawn")
                .arg("--dangerous-advanced")
                .arg("{\"foreground\":true}");
            // Pin the daemon to the same layout regardless of how this
            // client resolved it.
            cmd.env("OBJECTIVEAI_DIR", &layout.dir);
            cmd.env("OBJECTIVEAI_STATE", &layout.state);
            // The resident daemon is a per-state singleton with the
            // DEFAULT identity — scrub any agent/plugin identity from
            // this (possibly agent-invoked) process so it never leaks
            // into the long-lived daemon or everything it spawns. The
            // daemon then boots with `agent_instance_hierarchy =
            // "daemon"` and the rest unset; per-request identity
            // travels in the `/execute` envelope instead.
            for var in [
                "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
                "OBJECTIVEAI_AGENT_ID",
                "OBJECTIVEAI_AGENT_FULL_ID",
                "OBJECTIVEAI_AGENT_REMOTE",
                "OBJECTIVEAI_RESPONSE_ID",
                "OBJECTIVEAI_RESPONSE_IDS",
                "OBJECTIVEAI_PLUGIN_OWNER",
                "OBJECTIVEAI_PLUGIN_REPOSITORY",
                "OBJECTIVEAI_PLUGIN_VERSION",
            ] {
                cmd.env_remove(var);
            }
            // The daemon reads its bind config as bare `ADDRESS`/`PORT`/
            // `SECRET`. Hand it the `SECRET` from the CLI's `DAEMON_SECRET`
            // (or clear it), and scrub bare `ADDRESS`/`PORT` so a locally
            // spawned daemon uses its defaults and never inherits a stray
            // `$ADDRESS`/`$PORT` from the CLI's environment.
            match std::env::var("DAEMON_SECRET") {
                Ok(s) if !s.is_empty() => {
                    cmd.env("SECRET", s);
                }
                _ => {
                    cmd.env_remove("SECRET");
                }
            }
            cmd.env_remove("ADDRESS");
            cmd.env_remove("PORT");
        },
    )
    .await
    .map_err(|e| format!("ensure objectiveai-daemon: {e}"))?;

    // Local credential: an address-less `daemon` section describes
    // THIS daemon, and its secret is what the daemon's live auth
    // verifies against — DERIVE the signature from it (the stored
    // signature field is ignored: presenting a signature the secret
    // doesn't back would be pointless). No section at all falls back
    // to the `DAEMON_SIGNATURE` env.
    let signature = match &section {
        Some(section) => section
            .secret
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(derive_signature),
        None => daemon_signature_env(),
    };
    Ok((executor_for(&url, signature), agent_arguments_from_env()))
}

/// `daemon kill` — client-side. Signal the daemon-lock owner(s) directly
/// (never over the WS: the daemon can't kill itself mid-`/execute`).
/// Works whether the daemon is up (kills it) or down (nothing to kill).
/// Mirrors the former in-daemon `daemon kill` handler, now on this side.
async fn handle_daemon_kill(stdout: &mut tokio::io::Stdout) -> i32 {
    let layout = resolve_layout();
    let killed: usize = match objectiveai_sdk::lockfile::owners(&layout.lock_dir, DAEMON_LOCK_KEY)
        .await
    {
        Ok(pids) => pids.into_iter().map(objectiveai_sdk::process::kill_pid).sum(),
        Err(e) => {
            write_error_line(stdout, format!("read daemon lock owners: {e}"), Some(true)).await;
            return 1;
        }
    };
    write_json_line(
        stdout,
        &objectiveai_sdk::cli::command::daemon::kill::Response { killed },
    )
    .await;
    0
}

/// Build the per-request identity from this process's environment.
/// The hierarchy defaults to the CLI's own `"cli"` identity (the
/// daemon's own envelope-less default is `"daemon"`) when
/// `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY` is unset — a plain user
/// invocation. Every other unset field stays `None`, sent as no
/// header, which the daemon DELETES on the run's config — never
/// inherits.
fn agent_arguments_from_env() -> AgentArguments {
    let var = |key: &str| std::env::var(key).ok().filter(|s| !s.is_empty());
    AgentArguments {
        agent_instance_hierarchy: var("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY")
            .or_else(|| Some("cli".to_string())),
        agent_id: var("OBJECTIVEAI_AGENT_ID"),
        agent_full_id: var("OBJECTIVEAI_AGENT_FULL_ID"),
        agent_remote: var("OBJECTIVEAI_AGENT_REMOTE"),
        response_id: var("OBJECTIVEAI_RESPONSE_ID"),
        response_ids: var("OBJECTIVEAI_RESPONSE_IDS"),
    }
}

/// Did clap exit with a "successful informational output" variant?
/// `--help`, `--version`, or a missing-subcommand bail. Mirrors
/// `objectiveai_daemon::is_informational`.
fn is_informational(e: &clap::Error) -> bool {
    use clap::error::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// Render an execute-executor error to stdout. A daemon-framed
/// structured error (`Error::Cli`) is reserialized verbatim — it is
/// already the cli's `{"type":"error",...}` line shape. Transport /
/// decode / empty failures become a fresh fatal error line.
async fn write_execute_error(stdout: &mut tokio::io::Stdout, e: sse::Error) {
    match e {
        sse::Error::Cli(cli) => write_json_line(stdout, &cli).await,
        other => write_error_line(stdout, other.to_string(), Some(true)).await,
    }
}

async fn write_json_line<T: serde::Serialize>(stdout: &mut tokio::io::Stdout, value: &T) {
    let line = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => format!(r#"{{"type":"error","fatal":false,"message":"serialize error: {e}"}}"#),
    };
    let _ = stdout.write_all(line.as_bytes()).await;
    let _ = stdout.write_all(b"\n").await;
    let _ = stdout.flush().await;
}

async fn write_error_line(
    stdout: &mut tokio::io::Stdout,
    message: impl Into<String>,
    fatal: Option<bool>,
) {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal,
        message: serde_json::Value::String(message.into()),
    };
    write_json_line(stdout, &payload).await;
}

async fn write_help_line(stdout: &mut tokio::io::Stdout, help: &str) {
    let payload = serde_json::json!({ "type": "help", "help": help });
    write_json_line(stdout, &payload).await;
}
