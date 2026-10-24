//! Why a program could not be started or stopped.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

use diverge_sdk::file_lock;

use super::Program;
use crate::config;

/// What [`start`](super::start) and [`stop`](super::stop) fail with:
/// the lock, the binary, the child, the port, the exchange, or
/// stdout.
#[derive(Debug)]
pub enum Error {
    /// The root's lock could not be probed, or the lock file could not
    /// be read for its holder.
    Lock(file_lock::Error),
    /// A program already holds the root: the one named, by the process
    /// id it left in the lock file, where it left one.
    Running {
        /// Which program.
        program: Program,
        /// What its holder wrote in the lock file.
        holder: Option<String>,
    },
    /// Where this executable is could not be found, so the program's
    /// binary beside it could not be.
    Executable(io::Error),
    /// The log file could not be made, or its handle cloned.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The program's binary could not be spawned: not there, not
    /// runnable.
    Spawn {
        /// The binary.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// Whether the child had exited could not be asked.
    Wait(io::Error),
    /// The program exited before it held the root's lock: how, and
    /// what it wrote to its log.
    Died {
        /// Which program.
        program: Program,
        /// How it exited.
        status: ExitStatus,
        /// The whole of its log.
        log: String,
    },
    /// `cli.yaml` could not be read, or names no default daemon.
    Config(config::Error),
    /// `provider.yaml` could not be read as the provider reads it.
    Provider(diverge_provider::config::Error),
    /// `issuer.yaml` could not be read as the issuer reads it.
    Issuer(diverge_issuer::config::Error),
    /// The default daemon's address names a host that is not this
    /// one, and a stop is honoured only from the host the daemon is
    /// on.
    NotLocal(String),
    /// The default daemon's address carries a port that is not one.
    Address(String),
    /// Nothing is listening on the port: the program holds the root
    /// but has no listener yet, still starting.
    NotListening(u16),
    /// The port could not be connected to, for a reason other than
    /// nothing listening.
    Connect {
        /// The port.
        port: u16,
        /// Why.
        source: io::Error,
    },
    /// The socket failed under the request or the answer.
    Exchange(io::Error),
    /// The server closed the connection before any status line.
    NoAnswer,
    /// The first line of the answer is not an HTTP status line.
    Answer(String),
    /// The server refused the stop as not from this host.
    Refused,
    /// The server answered with a status that is neither the accept
    /// nor the refusal.
    Status(u16),
    /// Stdout could not be written.
    Stdout(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lock(error) => write!(f, "{error}"),
            Error::Running { program, holder } => match holder {
                Some(holder) => write!(
                    f,
                    "the {} is already running on this root, as process {holder}",
                    program.word()
                ),
                None => write!(f, "the {} is already running on this root", program.word()),
            },
            Error::Executable(error) => {
                write!(f, "where this executable is could not be found: {error}")
            }
            Error::Io { path, source } => {
                write!(f, "`{}` could not be used: {source}", path.display())
            }
            Error::Spawn { path, source } => {
                write!(f, "`{}` could not be started: {source}", path.display())
            }
            Error::Wait(error) => write!(f, "the child could not be waited on: {error}"),
            Error::Died {
                program,
                status,
                log,
            } => {
                write!(
                    f,
                    "the {} exited before it held the root, {status}",
                    program.word()
                )?;
                if log.trim().is_empty() {
                    write!(f, "; its log is empty")
                } else {
                    write!(f, "; its log:\n{}", log.trim_end())
                }
            }
            Error::Config(error) => write!(f, "{error}"),
            Error::Provider(error) => write!(f, "{error}"),
            Error::Issuer(error) => write!(f, "{error}"),
            Error::NotLocal(address) => write!(
                f,
                "the default daemon is at `{address}`, which is not this host, and a stop is honoured only from the daemon's own host"
            ),
            Error::Address(address) => {
                write!(
                    f,
                    "the default daemon's address `{address}` carries no port"
                )
            }
            Error::NotListening(port) => write!(
                f,
                "nothing is listening on port {port}: the program is not running, or is still starting"
            ),
            Error::Connect { port, source } => {
                write!(f, "port {port} could not be connected to: {source}")
            }
            Error::Exchange(error) => write!(f, "the request could not be exchanged: {error}"),
            Error::NoAnswer => write!(f, "the server closed the connection without answering"),
            Error::Answer(line) => write!(f, "the server's answer is not HTTP: `{line}`"),
            Error::Refused => write!(f, "the server refused the stop as not from this host"),
            Error::Status(code) => write!(f, "the server answered the stop with status {code}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Lock(error) => Some(error),
            Error::Executable(error)
            | Error::Wait(error)
            | Error::Exchange(error)
            | Error::Stdout(error) => Some(error),
            Error::Io { source, .. }
            | Error::Spawn { source, .. }
            | Error::Connect { source, .. } => Some(source),
            Error::Config(error) => Some(error),
            Error::Provider(error) => Some(error),
            Error::Issuer(error) => Some(error),
            Error::Running { .. }
            | Error::Died { .. }
            | Error::NotLocal(_)
            | Error::Address(_)
            | Error::NotListening(_)
            | Error::NoAnswer
            | Error::Answer(_)
            | Error::Refused
            | Error::Status(_) => None,
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
