//! A terminal: one shell on a pseudo-terminal, its output streamed and
//! its input taken at any time.

use std::io;
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
use std::os::unix::process::ExitStatusExt as _;
use std::process::Stdio;
use std::sync::Arc;

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::container_proxy::outside::endpoints::terminal::client::{channel_request, request};
use diverge_sdk::container_proxy::outside::endpoints::terminal::server::response::Frame;
use diverge_sdk::shared::containers::terminal::open::Open;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::io::unix::AsyncFd;
use tokio::process::Command;
use tokio::sync::Notify;

use crate::encode::encoded;

/// The shells tried, in order: the first the image has is the one.
const SHELLS: [&str; 2] = ["/bin/bash", "/bin/sh"];

/// Run a shell on a terminal and serve it, as
/// [`terminal`](diverge_sdk::shared::containers::terminal) states.
///
/// The rules:
///
/// 1. The shell is the first of [`SHELLS`] the image has, started in
///    `/` with `TERM=xterm-256color`, as the leader of a session of its
///    own whose controlling terminal is a new pseudo-terminal of the
///    request's size. An image with neither, or a terminal that cannot
///    be made, is one `Error`, and the finish.
/// 2. Every read of the terminal, up to [`CHUNK_SIZE`], is one `Stdout`
///    frame, each send waiting on the reader's room.
/// 3. Input is taken on its own, whatever the output is doing: each
///    channel request is `Stdin`, written to the terminal whole, in the
///    order the requests came; `Resize`, which sets the terminal's
///    size; or `Cancel`. Each is answered by a bare finish once done.
/// 4. When the shell exits, what remains in its process group is
///    killed, what the terminal still holds is sent, and `Exited` or
///    `Signaled` is last.
/// 5. A cancel, the client gone, or the reader abandoning the stream
///    kills the group at once: nothing more but the finish.
pub async fn terminal(scope: ScopeHandle, frame: request::Frame) {
    let scope = Arc::new(scope);
    let size = frame.0;
    // Rule 1.
    let mut shell = None;
    for candidate in SHELLS {
        if tokio::fs::metadata(candidate).await.is_ok_and(|meta| meta.is_file()) {
            shell = Some(candidate);
            break;
        }
    }
    let Some(shell) = shell else {
        return error(&scope, "shell", "the image has neither /bin/bash nor /bin/sh").await;
    };
    let (master, slave) = match open_pty(size) {
        Ok(pty) => pty,
        Err(reason) => return error(&scope, "pty", &reason.to_string()).await,
    };
    let mut command = Command::new(shell);
    command.current_dir("/").env("TERM", "xterm-256color").kill_on_drop(true);
    let stdio = |fd: &OwnedFd| fd.try_clone().map(Stdio::from);
    match (stdio(&slave), stdio(&slave), stdio(&slave)) {
        (Ok(stdin), Ok(stdout), Ok(stderr)) => {
            command.stdin(stdin).stdout(stdout).stderr(stderr);
        }
        _ => return error(&scope, "pty", "the terminal could not be shared with the shell").await,
    }
    // SAFETY: between fork and exec only async-signal-safe calls are
    // made: `setsid` and `ioctl`, on the child's own descriptor 0.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(reason) => return error(&scope, "spawn", &reason.to_string()).await,
    };
    // The shell holds the terminal now; this end's copy would keep it
    // open after the shell is gone.
    drop(slave);
    let group = child.id().and_then(|id| i32::try_from(id).ok());
    let master = match nonblocking(master) {
        Ok(master) => Arc::new(master),
        Err(reason) => {
            kill(group);
            let _ = child.wait().await;
            return error(&scope, "pty", &reason.to_string()).await;
        }
    };

    // Rule 3, on its own task.
    let stopped = Arc::new(Notify::new());
    let input = tokio::spawn(take_input(Arc::clone(&scope), Arc::clone(&master), Arc::clone(&stopped)));

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let status = loop {
        tokio::select! {
            // Rule 2.
            read = read_some(&master, &mut buffer) => match read {
                Ok(n) if n > 0 => send(&scope, &Frame::Stdout(&buffer[..n])).await,
                // The terminal hung up: the shell, and all it started, let it go.
                _ => break child.wait().await,
            },
            // Rule 4.
            waited = child.wait() => {
                kill(group);
                // What the terminal still holds, and no more: a job
                // left holding it is gone with the group.
                while let Ok(Some(n)) = read_now(&master, &mut buffer) {
                    if n == 0 {
                        break;
                    }
                    send(&scope, &Frame::Stdout(&buffer[..n])).await;
                }
                break waited;
            },
            // Rule 5.
            () = stopped.notified() => return stop(&scope, &mut child, group, input).await,
            () = scope.responses_gone() => return stop(&scope, &mut child, group, input).await,
        }
    };
    input.abort();
    let last = match status {
        Ok(status) => match (status.code(), status.signal()) {
            (Some(code), _) => Frame::Exited(code),
            (None, Some(signal)) => Frame::Signaled(signal),
            (None, None) => Frame::Error(failure("wait", "the shell ended with neither a code nor a signal")),
        },
        Err(reason) => Frame::Error(failure("wait", &reason.to_string())),
    };
    send(&scope, &last).await;
    scope.send_response_finish().await;
}

/// Rule 3: every channel request in order, until a cancel or the client
/// is gone, either of which tells `stopped`.
async fn take_input(scope: Arc<ScopeHandle>, master: Arc<AsyncFd<OwnedFd>>, stopped: Arc<Notify>) {
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Stdin(bytes)) => {
                // A write the shell cannot take now waits here, and so
                // does the next request: the client's pace is the
                // shell's.
                let _ = write_all(&master, bytes).await;
            }
            Ok(channel_request::Frame::Resize(size)) => {
                let _ = resize(&master, size);
            }
            Ok(channel_request::Frame::Cancel) => {
                scope.send_channel_response_finish(channel).await;
                break;
            }
            Err(_) => {}
        }
        scope.send_channel_response_finish(channel).await;
    }
    stopped.notify_one();
}

/// Rule 5: the group killed, the shell reaped, the input let go, and
/// the finish.
async fn stop(scope: &ScopeHandle, child: &mut tokio::process::Child, group: Option<i32>, input: tokio::task::JoinHandle<()>) {
    kill(group);
    let _ = child.wait().await;
    input.abort();
    scope.send_response_finish().await;
}

/// A new pseudo-terminal of `size`: its master, and the slave the shell
/// takes.
fn open_pty(size: Open) -> io::Result<(OwnedFd, OwnedFd)> {
    let mut master = -1;
    let mut slave = -1;
    let window = libc::winsize { ws_row: size.rows, ws_col: size.cols, ws_xpixel: 0, ws_ypixel: 0 };
    // SAFETY: `openpty` writes two descriptors into the integers given
    // and reads the window size; no name is asked for, nor any mode.
    let made = unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null(), &window) };
    if made != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: both descriptors are new and owned by nothing else.
    Ok(unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) })
}

/// The master, non-blocking and watched by the runtime, so reads and
/// writes wait without holding a thread and can be let go of.
fn nonblocking(master: OwnedFd) -> io::Result<AsyncFd<OwnedFd>> {
    let fd = master.as_raw_fd();
    // SAFETY: plain flag calls on a descriptor this end owns.
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags == -1 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    AsyncFd::new(master)
}

/// One read of the terminal, waiting for one. `Ok(0)` or an error
/// (`EIO` once nothing holds the other side) is its end.
async fn read_some(master: &AsyncFd<OwnedFd>, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        let mut ready = master.readable().await?;
        match ready.try_io(|fd| raw_read(fd.as_raw_fd(), buffer)) {
            Ok(result) => return result,
            Err(_would_block) => continue,
        }
    }
}

/// One read of what the terminal holds now: `None` when it holds
/// nothing, an error at its end.
fn read_now(master: &AsyncFd<OwnedFd>, buffer: &mut [u8]) -> io::Result<Option<usize>> {
    match raw_read(master.as_raw_fd(), buffer) {
        Ok(n) => Ok(Some(n)),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error),
    }
}

/// `read(2)`, as a result.
fn raw_read(fd: i32, buffer: &mut [u8]) -> io::Result<usize> {
    // SAFETY: the buffer is valid for its length, and the descriptor is
    // open for as long as the borrow of its owner lasts.
    let n = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
    if n < 0 { Err(io::Error::last_os_error()) } else { Ok(n as usize) }
}

/// Write all of `bytes` to the terminal, waiting for room as it needs.
async fn write_all(master: &AsyncFd<OwnedFd>, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let mut ready = master.writable().await?;
        // SAFETY: as `raw_read`, the other way.
        match ready.try_io(|fd| {
            let n = unsafe { libc::write(fd.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
            if n < 0 { Err(io::Error::last_os_error()) } else { Ok(n as usize) }
        }) {
            Ok(Ok(n)) => bytes = &bytes[n..],
            Ok(Err(error)) => return Err(error),
            Err(_would_block) => continue,
        }
    }
    Ok(())
}

/// Set the terminal's size; the shell's programs hear `SIGWINCH`.
fn resize(master: &AsyncFd<OwnedFd>, size: Open) -> io::Result<()> {
    let window = libc::winsize { ws_row: size.rows, ws_col: size.cols, ws_xpixel: 0, ws_ypixel: 0 };
    // SAFETY: `TIOCSWINSZ` reads one `winsize` from the pointer given.
    if unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ as _, &window) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Kill the shell's whole process group. A group already gone is nothing.
fn kill(group: Option<i32>) {
    if let Some(group) = group {
        // SAFETY: `killpg` takes plain integers and touches no memory.
        unsafe {
            libc::killpg(group, libc::SIGKILL);
        }
    }
}

/// One frame, as the scope's response.
async fn send(scope: &ScopeHandle, frame: &Frame<'_>) {
    if let Some(payload) = encoded(frame) {
        scope.send_response(&payload).await;
    }
}

/// An error in the proxy's words.
fn failure(kind: &str, reason: &str) -> Error {
    Error(serde_json::json!({ "kind": kind, "error": reason }))
}

/// Rule 1: the error, then the finish.
async fn error(scope: &ScopeHandle, kind: &str, reason: &str) {
    send(scope, &Frame::Error(failure(kind, reason))).await;
    scope.send_response_finish().await;
}
