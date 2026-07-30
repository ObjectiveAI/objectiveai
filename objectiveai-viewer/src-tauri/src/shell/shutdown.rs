//! The stdin graceful-shutdown listener — the viewer's half of the
//! daemon↔child control channel ([`objectiveai_sdk::child_stdio`]).
//!
//! When the daemon spawns the viewer (`viewer spawn`, installed binary
//! or `pnpm exec tauri dev` tree alike — stdin INHERITS down the dev
//! tree to this binary), it holds the viewer's stdin and kills it
//! GRACEFULLY, only ever by command: `viewer kill` (and every respawn
//! — config changes, development registrations) sends one acked
//! [`objectiveai_sdk::child_stdio::ChildStdioCommand::Shutdown`] line
//! and then waits, unbounded, for true process exit. There is no
//! signal and no force path — which is what guarantees every browser
//! tab gets to persist its profile to disk before the process dies.
//!
//! A viewer with no parent daemon is a FIRST-CLASS launch mode: stdin
//! may be a console, closed, or absent entirely. EOF, read errors,
//! and unparseable lines are therefore NEVER exit signals here — the
//! listener simply disarms on EOF/error and the viewer runs on
//! (daemon-death teardown is the OS job-object leash, not this
//! channel). Fully portable by construction: plain tokio line
//! reading, nothing platform-gated — the mobile builds compile this
//! unchanged and just see an instant EOF.

/// Spawn the resident stdin listener. Reads one JSON line at a time:
///
/// - `Shutdown` → ack FIRST (the daemon's ack-wait contract: the ack
///   means "teardown begun", exit is observed separately), then close
///   every live browser tab through the ordinary two-phase
///   cookie-flush path ([`crate::shell::browser::close_many`] — the
///   "tabs persist to disk" guarantee), then exit the app.
/// - `SetAddresses` → not ours (the laboratory host's dial list):
///   acked and ignored, so a misdirected command can never hang the
///   daemon's ack wait.
/// - Unparseable line → ignored, keep listening (nothing to
///   correlate an ack to; the daemon only writes well-formed lines).
/// - EOF / read error → disarm silently (see the module docs).
pub fn spawn_stdio_shutdown_listener(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let Some(request) =
                objectiveai_sdk::child_stdio::parse_child_stdio_request(&line)
            else {
                continue;
            };
            let shutdown = matches!(
                request.command,
                objectiveai_sdk::child_stdio::ChildStdioCommand::Shutdown
            );
            objectiveai_sdk::child_stdio::print_child_stdio_ack(
                &objectiveai_sdk::child_stdio::ChildStdioAck { id: request.id },
            );
            if shutdown {
                // Flush every browser to disk, then exit. `app.exit`
                // ends the event loop; `run` then shuts CEF down
                // behind it, exactly like the last-window exit path.
                let live = crate::shell::browser::live(&app).await;
                crate::shell::browser::close_many(&app, &live).await;
                app.exit(0);
                return;
            }
        }
    });
}
