use futures::StreamExt;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    // Before anything else: if this process was launched as a
    // subprocess-reaper guardian (macOS only), run its kqueue watch loop
    // and `process::exit`. A no-op on every other platform and on every
    // normal invocation.
    objectiveai_sdk::subprocess_reaper::run_guardian_if_invoked();

    // Two-tier dotenv, matching the api (objectiveai-api/src/main.rs):
    // the regular CWD `.env` overrides `<OBJECTIVEAI_DIR>/.env`. dotenv
    // never overrides an already-set var, so loading the CWD file FIRST
    // makes it win over the dir-scoped file, and the real environment
    // still wins over both. The dir-scoped file is the shared test/dev
    // config (`.objectiveai/.env`) — the source of the test credentials
    // and addresses the CLI and spawned api read.
    let _ = dotenv::dotenv();
    let dir = std::env::var_os("OBJECTIVEAI_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".objectiveai")
        });
    let _ = dotenv::from_path(dir.join(".env"));

    // Persist panics: the resident daemon runs detached with its
    // stderr discarded, so a panicking task (e.g. inside one hyper
    // connection's service future — which aborts just that connection
    // while the process lives) vanishes without a trace. Append every
    // panic to `<state_dir>/panics.log` — location, thread, payload —
    // and still chain to the default hook.
    {
        let state = std::env::var("OBJECTIVEAI_STATE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "default".to_string());
        let panic_log = dir.join("state").join(&state).join("panics.log");
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic payload>".to_string());
            let location = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "<unknown location>".to_string());
            let thread = std::thread::current()
                .name()
                .unwrap_or("<unnamed>")
                .to_string();
            let line = format!(
                "[pid {} thread {thread}] panicked at {location}: {payload}\n",
                std::process::id(),
            );
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&panic_log)
            {
                use std::io::Write as _;
                let _ = file.write_all(line.as_bytes());
            }
            default_hook(info);
        }));
    }

    // Windows-only: clear `HANDLE_FLAG_INHERIT` on this process's
    // stdio handles before any child spawns. See
    // `objectiveai_daemon::clear_stdio_inheritance` for the
    // grandchild-stdio-leak background. Still relevant: even
    // though there's no more `instance` subprocess, the
    // `stream=false` self-respawn for agents-spawn / functions-
    // execute spawns a detached cli child via `BinaryExecutor`,
    // and plugin RMCP servers spawned downstream from there
    // would inherit our stdio handles otherwise.
    #[cfg(windows)]
    objectiveai_daemon::clear_stdio_inheritance();

    let args: Vec<String> = std::env::args().collect();
    let code = run_command(args).await;
    std::process::exit(code);
}

async fn run_command(args: Vec<String>) -> i32 {
    let mut stdout = tokio::io::stdout();
    match objectiveai_daemon::run(args, None).await {
        // Both arms drain to stdout, one JSON line per item — `Execute`
        // yields typed root items, `ExecuteTransform` yields the
        // post-transform JSON; `drain` is generic over the item type.
        Ok(run_stream) => {
            let last_tool_exit = match run_stream {
                objectiveai_daemon::RunStream::Execute(stream) => drain(&mut stdout, stream).await,
                objectiveai_daemon::RunStream::ExecuteTransform(stream) => {
                    drain(&mut stdout, stream).await
                }
            };
            last_tool_exit.unwrap_or(0)
        }
        Err(e) => {
            // `--help` / `--version` / no-subcommand → render the
            // clap output as a single informational line. Exit 0 so
            // scripts pipelining `--help` aren't penalised.
            if let objectiveai_daemon::error::Error::ClapParse(ref clap_err) = e {
                if objectiveai_daemon::is_informational(clap_err) {
                    write_help_line(&mut stdout, &clap_err.to_string()).await;
                    return 0;
                }
            }
            write_error_line(&mut stdout, &e, Some(true)).await;
            match e {
                objectiveai_daemon::error::Error::ToolExit(code) => code,
                _ => 1,
            }
        }
    }
}

/// Drain a run stream to stdout — one JSON line per item. Returns the
/// last `ToolExit` code observed (surfaced as an `Err` item), if any.
/// Generic over the item type so it serves both `RunStream` arms.
async fn drain<S, T>(stdout: &mut tokio::io::Stdout, mut stream: S) -> Option<i32>
where
    S: futures::Stream<Item = Result<T, objectiveai_daemon::error::Error>> + Unpin,
    T: serde::Serialize,
{
    let mut last_tool_exit: Option<i32> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(value) => write_line(stdout, &value).await,
            Err(e) => {
                // `ToolExit(code)` carries the exit code the upstream
                // tool exited with — propagate it even though it's
                // surfaced as an Err item.
                if let objectiveai_daemon::error::Error::ToolExit(code) = &e {
                    last_tool_exit = Some(*code);
                }
                write_error_line(stdout, &e, None).await;
            }
        }
    }
    last_tool_exit
}

async fn write_line<T: serde::Serialize>(
    stdout: &mut tokio::io::Stdout,
    value: &T,
) {
    let line = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => format!(
            r#"{{"type":"error","fatal":false,"message":"serialize error: {e}"}}"#
        ),
    };
    let _ = stdout.write_all(line.as_bytes()).await;
    let _ = stdout.write_all(b"\n").await;
    let _ = stdout.flush().await;
}

async fn write_error_line(
    stdout: &mut tokio::io::Stdout,
    e: &objectiveai_daemon::error::Error,
    fatal: Option<bool>,
) {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal,
        message: e.output_message(),
    };
    write_line(stdout, &payload).await;
}

async fn write_help_line(stdout: &mut tokio::io::Stdout, help: &str) {
    let payload = serde_json::json!({
        "type": "help",
        "help": help,
    });
    write_line(stdout, &payload).await;
}
