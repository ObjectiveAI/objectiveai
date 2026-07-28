//! Socket-level policy shared by every ObjectiveAI server and
//! long-lived client connection: TCP keepalive.
//!
//! An idle TCP connection asserts nothing — a silently-dead peer
//! (machine power loss, vanished network path, expired NAT mapping)
//! is byte-for-byte indistinguishable from a healthy quiet one, and a
//! pure listener parked on it waits forever. Kernel keepalive probes
//! convert that silence into a socket error, which surfaces through
//! the exact same recv-error paths a graceful disconnect takes — no
//! application-level heartbeat frames, no timers in our tasks.
//!
//! One policy everywhere: probe after [`KEEPALIVE`] idle, re-probe at
//! the same interval, declare dead after [`KEEPALIVE_RETRIES`]
//! unanswered probes (retry count is kernel-fixed on some platforms).
//! 15s is the industry convention (axum's SSE keep-alive default,
//! reqwest's TCP keepalive default — which already covers every
//! reqwest-based SSE consumer in this workspace) and stays far under
//! the ~60s idle timeouts common in NATs and load balancers.

use std::time::Duration;

/// Idle time before the first probe AND the interval between probes.
pub const KEEPALIVE: Duration = Duration::from_secs(15);

/// Unanswered probes before the kernel declares the connection dead.
/// Best-effort: some platforms fix the count themselves (modern
/// Windows uses 10 — detection lands within ~2½ minutes there instead
/// of ~1).
pub const KEEPALIVE_RETRIES: u32 = 3;

/// Enable TCP keepalive (15s/15s/3) on a connected socket —
/// `tokio::net::TcpStream` or anything else exposing the raw socket.
/// Best-effort by design: a socket that cannot take the option (or is
/// already closing) is left as-is; the connection still works, it
/// just falls back to send-path-only death detection.
#[cfg(unix)]
pub fn set_tcp_keepalive<S: std::os::fd::AsFd>(stream: &S) {
    apply(socket2::SockRef::from(stream));
}

/// Enable TCP keepalive (15s/15s/3) on a connected socket —
/// `tokio::net::TcpStream` or anything else exposing the raw socket.
/// Best-effort by design: a socket that cannot take the option (or is
/// already closing) is left as-is; the connection still works, it
/// just falls back to send-path-only death detection.
#[cfg(windows)]
pub fn set_tcp_keepalive<S: std::os::windows::io::AsSocket>(stream: &S) {
    apply(socket2::SockRef::from(stream));
}

fn apply(socket: socket2::SockRef<'_>) {
    let keepalive = socket2::TcpKeepalive::new()
        .with_time(KEEPALIVE)
        .with_interval(KEEPALIVE)
        .with_retries(KEEPALIVE_RETRIES);
    let _ = socket.set_tcp_keepalive(&keepalive);
}
