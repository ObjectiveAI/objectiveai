//! An exec: one command run in the container, its output streamed.

use std::os::unix::process::ExitStatusExt as _;
use std::process::{ExitStatus, Stdio};

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::container_proxy::outside::endpoints::exec::client::request;
use diverge_sdk::container_proxy::outside::endpoints::exec::server::response::Frame;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::io::AsyncReadExt as _;
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

use crate::encode::encoded;
use crate::paths;

/// Run the command the request names and stream it, as
/// [`exec`](diverge_sdk::shared::containers::exec) states.
///
/// The rules:
///
/// 1. The argv runs as it is, no shell between: its first item a path,
///    or a name the container's `PATH` finds. Stdin is the null device;
///    stdout and stderr are pipes; the working directory is the
///    request's, else the proxy's own (the image's); the request's
///    variables are set over the container's environment. The command
///    is a process group of its own.
/// 2. A command that does not start — no argv, a working directory
///    that names no directory, a program not found or not executable —
///    is one `Error`, and the finish.
/// 3. Each read of a pipe, up to [`CHUNK_SIZE`], is one frame, in the
///    order the reads complete. Each send waits on the reader's room,
///    so a reader that falls behind fills the pipe and the command
///    waits on its write.
/// 4. When the command exits, anything it left running in its group is
///    killed — a process meant to outlive it detaches from the group
///    first — and the pipes are read to their end. `Exited` with the
///    code, or `Signaled` with the signal, is last.
/// 5. A cancel (any channel request), the client gone, or the reader
///    abandoning the stream kills the whole group at once: nothing more
///    is sent but the finish.
pub async fn exec(scope: ScopeHandle, frame: request::Frame) {
    let command = frame.0;
    // Rules 1 and 2.
    let Some((program, args)) = command.argv.split_first() else {
        return error(&scope, "argv", "the command names no program").await;
    };
    let mut process = Command::new(program);
    process
        .args(args)
        .envs(&command.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    if let Some(cwd) = &command.cwd {
        // No components is the root itself.
        let dir = if cwd.is_empty() { Some(std::path::PathBuf::from("/")) } else { paths::absolute(cwd) };
        let Some(dir) = dir else {
            return error(&scope, "cwd", "the working directory names no directory").await;
        };
        if !tokio::fs::metadata(&dir).await.is_ok_and(|meta| meta.is_dir()) {
            return error(&scope, "cwd", &format!("{} is not a directory", dir.display())).await;
        }
        process.current_dir(dir);
    }
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(reason) => return error(&scope, "spawn", &reason.to_string()).await,
    };
    let group = child.id();
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let mut out = vec![0; CHUNK_SIZE];
    let mut err = vec![0; CHUNK_SIZE];
    let mut status: Option<std::io::Result<ExitStatus>> = None;
    loop {
        if status.is_some() && stdout.is_none() && stderr.is_none() {
            break;
        }
        tokio::select! {
            // Rule 3.
            read = read_out(&mut stdout, &mut out), if stdout.is_some() => match read {
                Some(n) => send(&scope, &Frame::Stdout(&out[..n])).await,
                None => stdout = None,
            },
            read = read_err(&mut stderr, &mut err), if stderr.is_some() => match read {
                Some(n) => send(&scope, &Frame::Stderr(&err[..n])).await,
                None => stderr = None,
            },
            // Rule 4: the stragglers go, and the pipes close behind them.
            waited = child.wait(), if status.is_none() => {
                kill(group);
                status = Some(waited);
            },
            // Rule 5.
            _ = scope.recv_channel_request() => return stop(&scope, &mut child, group).await,
            () = scope.responses_gone() => return stop(&scope, &mut child, group).await,
        }
    }
    // Rule 4.
    let last = match status {
        Some(Ok(status)) => match (status.code(), status.signal()) {
            (Some(code), _) => Frame::Exited(code),
            (None, Some(signal)) => Frame::Signaled(signal),
            (None, None) => Frame::Error(failure("wait", "the command ended with neither a code nor a signal")),
        },
        Some(Err(reason)) => Frame::Error(failure("wait", &reason.to_string())),
        None => Frame::Error(failure("wait", "the command was never waited for")),
    };
    send(&scope, &last).await;
    scope.send_response_finish().await;
}

/// Rule 5: the group killed, the command reaped, and the finish.
async fn stop(scope: &ScopeHandle, child: &mut Child, group: Option<u32>) {
    kill(group);
    let _ = child.wait().await;
    scope.send_response_finish().await;
}

/// Kill the command's whole process group: it and everything it
/// started there. A group already gone is nothing.
fn kill(group: Option<u32>) {
    if let Some(group) = group.and_then(|group| i32::try_from(group).ok()) {
        // SAFETY: `killpg` takes plain integers and touches no memory.
        unsafe {
            libc::killpg(group, libc::SIGKILL);
        }
    }
}

/// One read of stdout: `Some(n)` bytes, `None` at its end or on a
/// failure, which ends the pipe the same way.
async fn read_out(pipe: &mut Option<ChildStdout>, buffer: &mut [u8]) -> Option<usize> {
    match pipe.as_mut()?.read(buffer).await {
        Ok(0) | Err(_) => None,
        Ok(n) => Some(n),
    }
}

/// One read of stderr, as [`read_out`].
async fn read_err(pipe: &mut Option<ChildStderr>, buffer: &mut [u8]) -> Option<usize> {
    match pipe.as_mut()?.read(buffer).await {
        Ok(0) | Err(_) => None,
        Ok(n) => Some(n),
    }
}

/// One frame, as the scope's response.
async fn send(scope: &ScopeHandle, frame: &Frame<'_>) {
    if let Some(payload) = encoded(frame) {
        scope.send_response(&payload).await;
    }
}

/// An error in the proxy's words: what it was doing, and why it failed.
fn failure(kind: &str, reason: &str) -> Error {
    Error(serde_json::json!({ "kind": kind, "error": reason }))
}

/// Rule 2: the error, then the finish.
async fn error(scope: &ScopeHandle, kind: &str, reason: &str) {
    send(scope, &Frame::Error(failure(kind, reason))).await;
    scope.send_response_finish().await;
}
