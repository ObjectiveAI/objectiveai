//! Shared process-lifecycle primitives for the `{viewer,api,mcp}
//! spawn|kill` cli subcommands.

use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Lowercase exe name (without `.exe`) → list of pids of running
/// processes matching it.
pub fn running_pids(exe_name: &str) -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    sys.processes()
        .values()
        .filter(|p| matches_exe(p.name().to_string_lossy().as_ref(), exe_name))
        .map(|p| p.pid().as_u32())
        .collect()
}

/// Returns Err if any process matching `exe_name` is currently running.
pub fn ensure_not_running(exe_name: &str) -> Result<(), crate::error::Error> {
    let pids = running_pids(exe_name);
    if pids.is_empty() {
        Ok(())
    } else {
        Err(crate::error::Error::AlreadyRunning {
            name: exe_name.to_string(),
            pids,
        })
    }
}

/// Send SIGTERM (Unix) / TerminateProcess (Windows) to every matching
/// pid. Returns the number of processes that were targeted. Idempotent:
/// a count of zero is not an error.
pub fn kill_by_name(exe_name: &str) -> usize {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    let mut count = 0usize;
    for process in sys.processes().values() {
        if matches_exe(process.name().to_string_lossy().as_ref(), exe_name) {
            // `kill_with(Term)` falls back to a hard kill on Windows
            // (no SIGTERM equivalent) and sends SIGTERM on Unix.
            let _ = process.kill_with(Signal::Term).or_else(|| Some(process.kill()));
            count += 1;
        }
    }
    count
}

/// Spawn `binary_path` with `ADDRESS=<address>` and `PORT=<port>` set in
/// its environment, then read its stderr line-by-line until a line
/// containing the substring "listening" (case-insensitive) shows up.
/// At that point the matched line is returned and the child is left
/// running detached — when the cli process exits, the spawned binary
/// is re-parented to init/launchd/services and keeps going.
///
/// If the child exits or closes stderr before announcing "listening",
/// returns [`crate::error::Error::SpawnNoListeningLine`].
pub async fn spawn_and_wait_for_listening(
    binary_path: &Path,
    address: &str,
    port: u16,
) -> Result<String, crate::error::Error> {
    let name = binary_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| binary_path.display().to_string());

    let mut cmd = Command::new(binary_path);
    cmd.env("ADDRESS", address)
        .env("PORT", port.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008)
        // — keep the spawned binary off the parent console and let it
        // outlive the cli.
        cmd.creation_flags(0x0800_0008);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::Error::Spawn(name.clone(), e))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| crate::error::Error::SpawnNoListeningLine { name: name.clone() })?;

    let mut reader = BufReader::new(stderr).lines();
    let mut listening_line: Option<String> = None;
    loop {
        tokio::select! {
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.to_ascii_lowercase().contains("listening") {
                            listening_line = Some(line);
                            break;
                        }
                    }
                    Ok(None) => break, // stderr closed
                    Err(_) => break,
                }
            }
            status = child.wait() => {
                // Child exited before announcing listening.
                let _ = status;
                break;
            }
        }
    }

    let Some(line) = listening_line else {
        return Err(crate::error::Error::SpawnNoListeningLine { name });
    };

    // Drain the rest of stderr in a fire-and-forget task so the child
    // doesn't EPIPE on its next write. The task ends when the child
    // closes stderr (or when the cli exits, whichever first).
    tokio::spawn(async move {
        while let Ok(Some(_)) = reader.next_line().await {}
    });

    // tokio's Child drops without killing (kill_on_drop is false by
    // default), so once `child` goes out of scope when this function
    // returns, the spawned binary is detached: on Unix the kernel
    // re-parents to init when the cli exits; on Windows the parent's
    // handle is released and the spawned binary continues.
    drop(child);

    Ok(line)
}

/// Case-insensitive match between an observed process name and a target
/// binary name. Strips a trailing `.exe` so the same target string
/// (`"objectiveai-api"`) works on every platform.
fn matches_exe(observed: &str, target: &str) -> bool {
    let trim = |s: &str| {
        s.strip_suffix(".exe")
            .or_else(|| s.strip_suffix(".EXE"))
            .unwrap_or(s)
            .to_ascii_lowercase()
    };
    trim(observed) == trim(target)
}
