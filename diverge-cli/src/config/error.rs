//! Why the root or the file could not be used, or a name not found.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What [`root`](super::root), [`load`](super::load) and
/// [`select`](super::select) fail with: the host, the filesystem, the
/// file's own content, or a daemon or an account the document does
/// not have.
#[derive(Debug)]
pub enum Error {
    /// No root was named and the host has no home directory to put
    /// one under.
    Home,
    /// The root could not be made, the working directory could not be
    /// read for its `.diverge`, or the file could not be read.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The file does not parse, or says something the document does
    /// not take.
    Parse {
        /// The file.
        path: PathBuf,
        /// Where in it, and what was wrong.
        source: serde_path_to_error::Error<serde_yaml_ng::Error>,
    },
    /// No daemon of the document has the name.
    NoDaemon(String),
    /// No account was named, and the daemon has no default.
    NoDefaultAccount(String),
    /// No account of the daemon has the name.
    NoAccount {
        /// The daemon.
        daemon: String,
        /// The account.
        account: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Home => write!(f, "no root was named and the host has no home directory"),
            Error::Io { path, source } => {
                write!(f, "`{}` could not be used: {source}", path.display())
            }
            Error::Parse { path, source } => {
                write!(f, "`{}` could not be read: {source}", path.display())
            }
            Error::NoDaemon(name) => write!(f, "no daemon is named `{name}`"),
            Error::NoDefaultAccount(daemon) => {
                write!(f, "no account was named, and the daemon `{daemon}` has no default")
            }
            Error::NoAccount { daemon, account } => {
                write!(f, "the daemon `{daemon}` has no account named `{account}`")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Home | Error::NoDaemon(_) | Error::NoDefaultAccount(_) | Error::NoAccount { .. } => None,
            Error::Io { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
        }
    }
}
