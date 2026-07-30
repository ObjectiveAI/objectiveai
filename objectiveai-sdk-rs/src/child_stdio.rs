//! The STDIN/STDOUT control channel between the daemon and its
//! stdio-speaking leashed children — the laboratory host and the
//! viewer. The daemon is the child's sole spawner and owns its pipes;
//! this vocabulary rides them, one JSON object per line.
//!
//! The daemon writes one [`ChildStdioRequest`] per stdin line, and the
//! child answers each PARSED line with one [`ChildStdioAck`] on
//! stdout, echoing the request's daemon-generated `id` — the
//! correlation key. Two commands exist:
//!
//! - [`ChildStdioCommand::SetAddresses`] — the laboratory host's
//!   DECLARATIVE dial list: the ENTIRE desired set of daemon
//!   connections, which the host CONVERGES to (connections absent from
//!   the list torn down, new ones dialed, changed-signature ones
//!   re-dialed, identical live ones untouched). The daemon converges
//!   the config-derived list right after the
//!   [`crate::process::ServerReady`] handshake and again on every
//!   `laboratories config` mutation. The ack confirms the list was
//!   CONVERGED (tasks spawned / cancelled), NOT connectivity: dialing
//!   retries forever. The viewer never receives this command (and
//!   would ack-and-ignore it — every parsed line is acked, so the
//!   daemon's ack wait can never hang on a live child).
//! - [`ChildStdioCommand::Shutdown`] — the GRACEFUL kill, both
//!   children: ack first (shutdown begun), then flush and exit. The
//!   host stops its started containers; the viewer closes every
//!   browser tab (persisting its profile to disk) before exiting. This
//!   is the ONLY kill the daemon sends these children — no signal, no
//!   force path; the daemon waits, unbounded, for true process exit.
//!
//! Stdin EOF remains the host's shutdown backstop (entry drop, daemon
//! death). The viewer, by contrast, IGNORES EOF entirely: a viewer
//! launched by hand — no parent, no pipes — is a first-class mode,
//! and its stdin listener simply disarms when the line stream ends.
//!
//! This is a different transport from the `/laboratory` WebSocket
//! vocabulary and deliberately NAIVE to it. Like
//! [`crate::process::ServerReady`], these are internal pipe types:
//! plain serde, no schemas.

/// One control request, daemon → child, one JSON object per stdin
/// line: a daemon-generated `id` (echoed back in the ack) wrapped
/// around the [`ChildStdioCommand`], flattened — on the wire the line
/// is `{"id": …, "type": …, …}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChildStdioRequest {
    pub id: String,
    #[serde(flatten)]
    pub command: ChildStdioCommand,
}

/// The command a [`ChildStdioRequest`] carries.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChildStdioCommand {
    /// The ENTIRE desired dial list — the laboratory host converges
    /// its live connections to exactly this set (see the module docs).
    /// An empty list is legal: the host idles with zero connections.
    /// Duplicate addresses: last entry wins.
    SetAddresses { addresses: Vec<DialEntry> },
    /// Graceful shutdown: ack, flush durable state (containers
    /// stopped / browser profiles persisted), exit. See the module
    /// docs.
    Shutdown,
}

/// One desired dial-list entry: `address` is a daemon `http://` base
/// (the host appends `/laboratory`); `signature` is presented in the
/// auth preamble (`None` ⇒ unauthenticated).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DialEntry {
    pub address: String,
    pub signature: Option<String>,
}

/// The child's answer to one [`ChildStdioRequest`], child → daemon,
/// one JSON object per stdout line: the request's `id`, echoed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChildStdioAck {
    pub id: String,
}

/// Parse one stdin line as a control request. `None` for anything
/// else (the child ignores unparseable lines; with nothing to
/// correlate, the daemon's ack wait would hang — the daemon only ever
/// writes well-formed lines).
pub fn parse_child_stdio_request(line: &str) -> Option<ChildStdioRequest> {
    serde_json::from_str(line.trim()).ok()
}

/// Parse one stdout line as an ack. `None` for anything else (the
/// ready line, stray output — the daemon's reader skips them).
pub fn parse_child_stdio_ack(line: &str) -> Option<ChildStdioAck> {
    serde_json::from_str(line.trim()).ok()
}

/// Print one ack line to stdout and flush — the child side of the
/// channel, sharing stdout with the ready line under the same
/// lock-and-flush discipline as [`crate::process::print_ready`].
pub fn print_child_stdio_ack(ack: &ChildStdioAck) {
    use std::io::Write;
    let line = serde_json::to_string(ack).expect("ChildStdioAck serializes");
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}
