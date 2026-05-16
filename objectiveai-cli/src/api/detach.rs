/// Re-invokes the current CLI as a subprocess with `--detach` removed from
/// the arguments. Emits `Detached { pid }`, then for every line the
/// orphan writes to its stdout, parses the line as an [`Output`] and
/// re-emits it through `Output::emit(handle)` — so the routing logic
/// (handle vs stdout, fatal-error stderr mirror) lives in one place
/// instead of being duplicated here. A non-JSONL line on the orphan's
/// stdout is a contract violation and panics.
///
/// Stderr is forwarded raw to this process's own stderr (a separate
/// channel from `handle`'s child stdin — the embedder captures cli's
/// stderr pipe independently if it wants diagnostic visibility into
/// the orphan).
///
/// Once `log_stream_ready` appears on the orphan's stdout, exits with
/// code 0; the orphan continues running and writing more lines, but
/// nobody reads them. If the orphan exits without producing the
/// handshake, forwards its exit code.
///
/// The orphan has no idea about `handle` — it's a fresh CLI invocation
/// with the default `None` handle, writing JSONL to its own stdout.
/// The parent's forwarding loop is the only place the JSONL stream
/// gets routed to the embedder.
///
/// [`Output`]: objectiveai_sdk::cli::output::Output
pub async fn detach(handle: &objectiveai_sdk::cli::output::Handle) -> ! {
    let exe = std::env::current_exe().expect("failed to get current executable path");
    let args: Vec<String> = std::env::args()
        .skip(1) // skip binary name
        .filter(|a| a != "--detach")
        .collect();

    let mut cmd = tokio::process::Command::new(exe);
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // On Windows, create the child in a new process group and detach it from
    // the parent's job object so it survives when the parent exits.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    let mut child = cmd.spawn().expect("failed to spawn detached process");

    let pid = child.id().expect("failed to get child PID");
    objectiveai_sdk::cli::output::Output::<objectiveai_sdk::cli::output::Detached>::Notification(objectiveai_sdk::cli::output::Notification { value: 
        objectiveai_sdk::cli::output::Detached { pid },
     })
    .emit(handle).await;

    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let mut stdout_reader = tokio::io::BufReader::new(child_stdout);
    let mut stderr_reader = tokio::io::BufReader::new(child_stderr);
    let mut stdout_line = String::new();
    let mut stderr_line = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    loop {
        tokio::select! {
            result = tokio::io::AsyncBufReadExt::read_line(&mut stdout_reader, &mut stdout_line), if !stdout_done => {
                let n = result.unwrap_or(0);
                if n == 0 {
                    stdout_done = true;
                } else {
                    // Parse each orphan-stdout line as an Output and run
                    // it through the standard emit pipeline. That keeps
                    // all routing logic (handle vs stdout, fatal-error
                    // stderr mirror) in one place — Output::emit — and
                    // never deals with raw bytes here.
                    //
                    // The cli is documented to produce only JSONL on
                    // stdout, so a parse failure is a contract violation
                    // — panic so it's loud and traceable rather than
                    // silently corrupting the consumer's stream.
                    let trimmed = stdout_line.trim_end_matches(['\r', '\n']);
                    let out: objectiveai_sdk::cli::output::Output<serde_json::Value> =
                        serde_json::from_str(trimmed)
                            .expect("orphan stdout produced a non-JSONL line");
                    out.emit(handle).await;
                    if crate::log_line::parse_log_stream_ready(&stdout_line).is_some() {
                        std::process::exit(0);
                    }
                    stdout_line.clear();
                }
            }
            result = tokio::io::AsyncBufReadExt::read_line(&mut stderr_reader, &mut stderr_line), if !stderr_done => {
                let n = result.unwrap_or(0);
                if n == 0 {
                    stderr_done = true;
                } else {
                    eprint!("{stderr_line}");
                    stderr_line.clear();
                }
            }
        }
        if stdout_done && stderr_done {
            break;
        }
    }

    // Never saw the log line — child must have failed. Forward its exit code.
    let status = child.wait().await.expect("failed to wait for child");
    std::process::exit(status.code().unwrap_or(1))
}
