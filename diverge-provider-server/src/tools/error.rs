//! Why a program did not do what it was run for.

use std::fmt;
use std::io;
use std::process::ExitStatus;

/// A program that could not be started, or that ran and refused.
#[derive(Debug)]
pub enum Error {
    /// The program could not be started: not on this host's `PATH`,
    /// or not runnable. Where a tool runs inside the podman machine,
    /// the program that could not be started is `podman` itself.
    Spawn {
        /// The program, as it was named.
        program: String,
        /// Why.
        source: io::Error,
    },
    /// The program ran and exited with a status the caller does not
    /// accept, and this is what it wrote to stderr.
    Status {
        /// The program, as it was named.
        program: String,
        /// How it exited.
        status: ExitStatus,
        /// What it wrote to stderr, whole.
        stderr: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spawn { program, source } => write!(f, "{program} could not be started: {source}"),
            Error::Status { program, status, stderr } => {
                write!(f, "{program} refused, {status}: {}", stderr.trim_end())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Spawn { source, .. } => Some(source),
            Error::Status { .. } => None,
        }
    }
}
