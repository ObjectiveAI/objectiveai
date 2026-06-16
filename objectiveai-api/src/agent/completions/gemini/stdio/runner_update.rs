use super::super::GeminiEvent;
use super::{StdioDiagLevel, StdioEndStatus};

/// One thing that happened for an in-flight request, delivered via
/// the per-request mpsc channel.
#[derive(Debug)]
pub enum RunnerUpdate {
    /// One stdout `event` line, fully parsed.
    Event(GeminiEvent),
    /// Terminal `end` line — emitted exactly once per accepted run.
    /// Receivers should treat the channel as closed after this.
    End(StdioEndStatus),
    /// One stderr `diag` line for this request.
    Diag {
        level: StdioDiagLevel,
        message: String,
    },
    /// Process-level fatal — the runner is exiting non-zero. Fanned
    /// out to every in-flight request as a courtesy. Receivers should
    /// fail their stream with this message.
    Fatal(String),
    /// The runner subprocess exited (or its IO closed) without ever
    /// emitting an `end` for this request. Receivers should fail
    /// with a runner-died error.
    RunnerExited,
}
