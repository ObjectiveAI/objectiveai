//! The daemon → viewer stdin channel: development-plugin registrations.
//!
//! The VIEWER twin of the laboratory host's dial-list channel
//! ([`crate::laboratories::daemon::stdio`]), sharing its whole
//! doctrine:
//!
//! - **Declarative.** One command carries the ENTIRE desired state —
//!   every viewer plugin currently registered for development — and
//!   the viewer converges: arms watchers for new entries, drops them
//!   for absent ones. Convergence is idempotent by construction.
//! - **Acked by id.** The ack confirms APPLICATION, not effect. The
//!   ack's wire shape is identical to the host channel's (`{"id":…}`),
//!   which lets the daemon's one stdio ack reader serve both children.
//! - **EOF is the graceful-shutdown signal** — for a viewer built with
//!   its `stdio` feature, the daemon closing stdin means "exit".
//!
//! Like `ServerReady` and the host stdio types, these are internal
//! pipe types: plain serde, no schemas.

/// One daemon → viewer command, one JSON object per stdin line:
/// `{"id": …, "type": …, …}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViewerStdioRequest {
    /// Correlation id, echoed verbatim in the ack. The daemon mints a
    /// UUID; the viewer treats it as opaque.
    pub id: String,
    #[serde(flatten)]
    pub command: ViewerStdioCommand,
}

/// What the daemon can tell the viewer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewerStdioCommand {
    /// The COMPLETE set of viewer plugins registered for development.
    /// Not a delta: the viewer serves `plugin://` for exactly these
    /// trios from their directories, and for nothing else.
    SetDevelopmentPlugins {
        plugins: Vec<DevelopmentViewerPlugin>,
    },
}

/// One development registration: the canonical trio (owner and name
/// lowercased, version verbatim — the daemon registry's own
/// canonicalization) and the ABSOLUTE path of the plugin's source
/// directory, the one holding `objectiveai.json`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DevelopmentViewerPlugin {
    pub owner: String,
    pub name: String,
    pub version: String,
    pub path: String,
}

/// The viewer's answer to one [`ViewerStdioRequest`], viewer → daemon,
/// one JSON object per stdout line: the request's `id`, echoed. Wire
/// shape deliberately identical to the host channel's ack.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ViewerStdioAck {
    pub id: String,
}

/// Parse one stdin line as a viewer stdio request. `None` for anything
/// else (unparseable lines are ignored; with nothing to correlate, the
/// daemon's bounded ack wait elapses and it moves on).
pub fn parse_viewer_stdio_request(line: &str) -> Option<ViewerStdioRequest> {
    serde_json::from_str(line.trim()).ok()
}

/// Print one ack line to stdout and flush — the viewer side of the
/// channel, sharing stdout with the ready line under the same
/// lock-and-flush discipline as [`crate::process::print_ready`].
pub fn print_viewer_stdio_ack(ack: &ViewerStdioAck) {
    use std::io::Write;
    let line = serde_json::to_string(ack).expect("ViewerStdioAck serializes");
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}
