//! What a run of a hook fails with.

use std::fmt;
use std::io;
use std::process::ExitStatus;

/// A hook that did not answer.
///
/// The first five are this end's: the command could not be run, or
/// the question could not be written. [`Status`](Self::Status) is the
/// hook's own: it exited non-zero, which is how a hook says no and
/// also how one fails, and the caller of a yes-or-no question reads it
/// as no. [`Parse`](Self::Parse) is a hook that exited `0` with a
/// stdout that is not the expected answer, and it carries the path
/// inside the document where the mismatch is.
#[derive(Debug)]
pub enum Error {
    /// The array has no program in it.
    Empty,
    /// The question would not serialize as JSON.
    Encode(serde_json::Error),
    /// The program could not be started.
    Spawn {
        /// The program as it was resolved.
        program: String,
        /// Why the OS would not start it.
        source: io::Error,
    },
    /// The question could not be written to the program's stdin.
    Stdin(io::Error),
    /// The program could not be waited for.
    Wait(io::Error),
    /// The program exited non-zero: no, or a failure of its own.
    Status {
        /// How it exited.
        status: ExitStatus,
        /// What it wrote to stderr, lossily decoded.
        stderr: String,
    },
    /// The program exited `0`, and its stdout is not the answer's type.
    Parse(serde_path_to_error::Error<serde_json::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => f.write_str("the hook names no program"),
            Error::Encode(error) => {
                write!(f, "the question did not serialize: {error}")
            }
            Error::Spawn { program, source } => {
                write!(f, "the hook `{program}` could not be started: {source}")
            }
            Error::Stdin(error) => {
                write!(f, "the question could not be written to the hook: {error}")
            }
            Error::Wait(error) => {
                write!(f, "the hook could not be waited for: {error}")
            }
            Error::Status { status, stderr } => {
                write!(f, "the hook exited with {status}")?;
                if !stderr.is_empty() {
                    write!(f, ": {}", stderr.trim_end())?;
                }
                Ok(())
            }
            Error::Parse(error) => write!(
                f,
                "the hook's answer did not parse at `{}`: {}",
                error.path(),
                error.inner()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Encode(error) => Some(error),
            Error::Spawn { source, .. } => Some(source),
            Error::Stdin(error) | Error::Wait(error) => Some(error),
            Error::Parse(error) => Some(error),
            Error::Empty | Error::Status { .. } => None,
        }
    }
}
