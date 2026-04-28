use std::fmt;

/// Raised when a turn is cancelled via an [`super::AbortSignal`]. Mirrors
/// `AbortError` in `abort.py:9-10`.
///
/// Note: Python's `AbortError` inherits from `Exception` directly (not
/// `CodexSdkError`), so it lives here as a standalone struct rather than as
/// a variant of [`super::super::Error`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbortError {
    pub message: String,
}

impl fmt::Display for AbortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AbortError {}
