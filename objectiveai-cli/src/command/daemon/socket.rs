//! Local-socket plumbing for the plugin daemon, built on the
//! `interprocess` crate (AF_UNIX on Unix, named pipes on Windows).
//!
//! Each `daemon: true` plugin gets one socket; the daemon binds it and
//! bridges every connection's single request line straight to that
//! plugin's stdin (as JSONL), then acks. `plugins daemon notify`
//! connects to the same socket and sends one line. The socket lives in
//! the plugin's own per-state scratch dir
//! (`<state>/plugins/<owner>/<name>/<version>/daemon.sock`) on Unix; on
//! Windows — where named pipes are not filesystem entries — the same
//! coordinates derive a stable pipe name so both sides agree.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{ListenerOptions, Name};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

pub use interprocess::local_socket::tokio::{Listener, Stream};

/// Per-state daemon singleton lock key (under `state_dir/locks`).
pub const DAEMON_LOCK_KEY: &str = "plugins-daemon";

/// Init gate key — serializes daemon startup so the final lock is only
/// published after every plugin is spawned and every socket is bound
/// (mirrors `objectiveai-db`'s `db-init` gate).
pub const DAEMON_INIT_LOCK_KEY: &str = "plugins-daemon-init";

/// The per-plugin socket path:
/// `<state>/plugins/<owner>/<name>/<version>/daemon.sock`.
pub fn plugin_socket_path(state_dir: &Path, owner: &str, name: &str, version: &str) -> PathBuf {
    state_dir
        .join("plugins")
        .join(owner)
        .join(name)
        .join(version)
        .join("daemon.sock")
}

#[cfg(unix)]
fn socket_name(socket_path: &Path) -> std::io::Result<Name<'static>> {
    use interprocess::local_socket::{GenericFilePath, ToFsName};
    socket_path
        .to_path_buf()
        .into_os_string()
        .to_fs_name::<GenericFilePath>()
}

#[cfg(windows)]
fn socket_name(socket_path: &Path) -> std::io::Result<Name<'static>> {
    use interprocess::local_socket::{GenericNamespaced, ToNsName};
    // Named pipes are not filesystem entries; derive a stable pipe name
    // from the full socket path so the daemon (bind) and notify
    // (connect) always agree on it.
    let hash = twox_hash::XxHash3_128::oneshot(socket_path.to_string_lossy().as_bytes());
    format!("objectiveai-daemon-{hash:032x}.sock").to_ns_name::<GenericNamespaced>()
}

/// Bind a listener for a plugin's daemon socket. Sockets are never
/// proactively cleaned up, so a stale file from a previous daemon may
/// already be on disk; on Unix we reclaim it here (an existing socket
/// file makes `bind` fail with `EADDRINUSE`). This is the one place that
/// accounts for the file already existing — everywhere else just leaves
/// sockets be.
pub fn bind(socket_path: &Path) -> std::io::Result<Listener> {
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(socket_path);
    }
    let name = socket_name(socket_path)?;
    ListenerOptions::new().name(name).create_tokio()
}

/// Connect to a plugin's daemon socket.
pub async fn connect(socket_path: &Path) -> std::io::Result<Stream> {
    let name = socket_name(socket_path)?;
    Stream::connect(name).await
}

/// Accept connections forever, bridging each one's first line into the
/// plugin's stdin. Runs until the listener errors (e.g. the daemon is
/// shutting down).
pub async fn accept_loop(listener: Listener, stdin: Arc<Mutex<ChildStdin>>) {
    loop {
        match listener.accept().await {
            Ok(conn) => {
                tokio::spawn(handle_conn(conn, stdin.clone()));
            }
            Err(_) => break,
        }
    }
}

/// One connection: read exactly one request line, write it to the
/// plugin's stdin as a JSON line, then ack `{"ok":true}` (or
/// `{"ok":false}` if the stdin write failed — i.e. the plugin is gone).
async fn handle_conn(conn: Stream, stdin: Arc<Mutex<ChildStdin>>) {
    let (read_half, mut write_half) = tokio::io::split(conn);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return;
    }
    let payload = line.trim_end_matches(['\r', '\n']);
    if payload.is_empty() {
        return;
    }

    let forwarded = {
        let mut guard = stdin.lock().await;
        guard.write_all(payload.as_bytes()).await.is_ok()
            && guard.write_all(b"\n").await.is_ok()
            && guard.flush().await.is_ok()
    };

    let ack: &[u8] = if forwarded {
        b"{\"ok\":true}\n"
    } else {
        b"{\"ok\":false}\n"
    };
    let _ = write_half.write_all(ack).await;
    let _ = write_half.flush().await;
}
