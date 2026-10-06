//! Why the postmaster could not be stopped, started, or reached.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// What [`stop`](super::stop), [`start`](super::start) and
/// [`ready`](super::ready) fail with.
#[derive(Debug)]
pub enum Error {
    /// `postgres` or `pg_ctl` could not be started.
    Spawn {
        /// The program, as it was named.
        program: String,
        /// Why.
        source: io::Error,
    },
    /// `pg_ctl` ran and exited with a status that is neither of the
    /// answers the question has, and this is what it wrote to stderr.
    PgCtl {
        /// What it was asked.
        action: String,
        /// How it exited.
        status: ExitStatus,
        /// What it wrote to stderr, whole.
        stderr: String,
    },
    /// The postmaster was asked to stop fast, then immediately, then
    /// signalled, and still runs.
    StillRunning,
    /// The pid file could not be removed.
    Io {
        /// The file.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// No loopback port could be found free.
    Port(io::Error),
    /// Whether the child had exited could not be asked.
    Wait(io::Error),
    /// The postmaster exited before it was ready, and this is how.
    Exited(ExitStatus),
    /// The postmaster did not accept on its port in time.
    Unready(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spawn { program, source } => write!(f, "{program} could not be started: {source}"),
            Error::PgCtl { action, status, stderr } => {
                write!(f, "pg_ctl {action} failed, {status}: {}", stderr.trim_end())
            }
            Error::StillRunning => write!(f, "the postmaster still runs after a fast stop, an immediate stop and a kill"),
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Port(error) => write!(f, "no loopback port could be had: {error}"),
            Error::Wait(error) => write!(f, "the postmaster could not be waited on: {error}"),
            Error::Exited(status) => write!(f, "the postmaster exited before it was ready: {status}"),
            Error::Unready(port) => write!(f, "the postmaster did not accept on 127.0.0.1:{port} in time"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Spawn { source, .. } | Error::Io { source, .. } => Some(source),
            Error::Port(error) | Error::Wait(error) => Some(error),
            Error::PgCtl { .. } | Error::StillRunning | Error::Exited(_) | Error::Unready(_) => None,
        }
    }
}
