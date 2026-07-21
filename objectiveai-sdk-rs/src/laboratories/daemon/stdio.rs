//! The laboratory host's STDIN/STDOUT control channel — the dial-list
//! vocabulary between the daemon (the host's sole spawner, which owns
//! its pipes) and the host binary.
//!
//! The host is born with ZERO daemon connections: no `--address` argv
//! exists. The protocol is DECLARATIVE — one command,
//! [`HostStdioCommand::SetAddresses`], carries the ENTIRE desired dial
//! list, and the host CONVERGES to it: connections absent from the
//! list are torn down, new ones dialed, changed-signature ones
//! re-dialed, identical live ones left untouched. The daemon writes
//! one [`HostStdioRequest`] JSON object per stdin line (converging the
//! config-derived list right after the [`crate::process::ServerReady`]
//! handshake, and again on every `laboratories config` mutation), and
//! the host answers each request with one [`HostStdioAck`] line on
//! stdout, echoing the request's daemon-generated `id` — the
//! correlation key.
//!
//! An ack confirms the dial list was CONVERGED (tasks spawned /
//! cancelled), NOT connectivity: dialing retries forever, and
//! connection success is observed through the daemon's registry
//! exactly as before. Convergence is idempotent by construction — the
//! same list twice is a no-op, and the host NEVER holds two
//! connections to one address.
//!
//! This is a different transport from the `/laboratory` WebSocket
//! vocabulary in the sibling modules, and deliberately NAIVE to it.
//! Like [`crate::process::ServerReady`], these are internal pipe
//! types: plain serde, no schemas.

/// One dial-list request, daemon → host, one JSON object per stdin
/// line: a daemon-generated `id` (echoed back in the ack) wrapped
/// around the [`HostStdioCommand`], flattened — on the wire the line
/// is `{"id": …, "type": …, …}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostStdioRequest {
    pub id: String,
    #[serde(flatten)]
    pub command: HostStdioCommand,
}

/// The command a [`HostStdioRequest`] carries.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostStdioCommand {
    /// The ENTIRE desired dial list — the host converges its live
    /// connections to exactly this set (see the module docs). An empty
    /// list is legal: the host idles with zero connections. Duplicate
    /// addresses: last entry wins.
    SetAddresses { addresses: Vec<DialEntry> },
}

/// One desired dial-list entry: `address` is a daemon `http://` base
/// (the host appends `/laboratory`); `signature` is presented in the
/// auth preamble (`None` ⇒ unauthenticated).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DialEntry {
    pub address: String,
    pub signature: Option<String>,
}

/// The host's answer to one [`HostStdioRequest`], host → daemon, one
/// JSON object per stdout line: the request's `id`, echoed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostStdioAck {
    pub id: String,
}

/// Parse one stdin line as a dial-list request. `None` for anything
/// else (the host ignores unparseable lines; with nothing to
/// correlate, the daemon's ack wait times out and reports the error).
pub fn parse_host_stdio_request(line: &str) -> Option<HostStdioRequest> {
    serde_json::from_str(line.trim()).ok()
}

/// Parse one stdout line as an ack. `None` for anything else (the
/// ready line, stray output — the daemon's reader skips them).
pub fn parse_host_stdio_ack(line: &str) -> Option<HostStdioAck> {
    serde_json::from_str(line.trim()).ok()
}

/// Print one ack line to stdout and flush — the host side of the
/// channel, sharing stdout with the ready line under the same
/// lock-and-flush discipline as [`crate::process::print_ready`].
pub fn print_host_stdio_ack(ack: &HostStdioAck) {
    use std::io::Write;
    let line = serde_json::to_string(ack).expect("HostStdioAck serializes");
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}
