use thiserror::Error;

/// Local errors raised while parsing wire data or dealing with the codex
/// subprocess. Mirrors the `CodexSdkError` family in the Python SDK
/// (`errors.py`) — every Python subclass of `CodexSdkError` has a
/// corresponding variant here.
///
/// The base `CodexSdkError` exception class itself is represented by this
/// enum. `AbortError` is intentionally *not* a variant — Python's
/// `AbortError` inherits from `Exception` directly (not `CodexSdkError`),
/// so it lives as a standalone struct in [`super::AbortError`].
#[derive(Debug, Error)]
pub enum Error {
    /// A JSONL line could not be parsed into a [`super::ThreadEvent`].
    /// Mirrors `EventParseError` in `errors.py:12-13`.
    #[error("failed to parse thread event line: {0}")]
    EventParse(#[from] serde_json::Error),

    /// The codex subprocess exited with a non-zero status. The string is the
    /// captured stderr. Mirrors `CodexExecError` in `errors.py:16-17`.
    #[error("codex exec failed: {0}")]
    Exec(String),

    /// The runner emitted a `turn.failed` event; the inner string is the
    /// `error.message` payload. Mirrors `ThreadRunError` in `errors.py:8-9`.
    #[error("thread run error: {0}")]
    ThreadRun(String),

    /// Installing the codex CLI binary failed. Mirrors `CodexInstallError`
    /// in `errors.py:20-21`.
    #[error("codex install failed: {0}")]
    Install(String),

    /// Reading or writing Codex `auth.json` failed. Mirrors `CodexAuthError`
    /// in `errors.py:24-25`.
    #[error("codex auth failed: {0}")]
    Auth(String),
}
